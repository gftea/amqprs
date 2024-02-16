//! Implementation of AMQP_0-9-1's Connection class compatible with RabbitMQ.
//!
//! It provides [APIs][`Connection`] to manage an AMQP `Connection`.
//!
//! User should hold the connection object until no longer needs it, and call the [`close`] method
//! to gracefully shutdown the connection.
//!
//! When connection object is dropped, it will try with best effort
//! to close the connection, but no guarantee to handle close errors.
//!
//! # Example
//! ```rust
//! # use amqprs::security::SecurityCredentials;
//! # use amqprs::connection::{OpenConnectionArguments, Connection};
//! # use amqprs::callbacks;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");
//! // open a connection with given arguments
//! let connection = Connection::open(&args).await.unwrap();
//!
//! // register callback for handling asynchronous message from server for this connection
//! connection.register_callback(callbacks::DefaultConnectionCallback).await.unwrap();
//!
//! // ... use the connection ...
//!
//! // gracefully shutdown and consume the connection
//! connection.close().await.unwrap();
//!
//! # }
//! ```
//! [`Connection`]: struct.Connection.html
//! [`Channel`]: ../channel/struct.Channel.html
//! [`close`]: struct.Connection.html#method.close

use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use amqp_serde::types::{
    AmqpChannelId, AmqpPeerProperties, FieldTable, FieldValue, LongStr, LongUint, ShortUint,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::{
    frame::{
        Blocked, Close, CloseOk, Frame, MethodHeader, Open, OpenChannel, OpenChannelOk,
        ProtocolHeader, StartOk, TuneOk, Unblocked, DEFAULT_CONN_CHANNEL, FRAME_MIN_SIZE,
    },
    net::{
        ChannelResource, ConnManagementCommand, IncomingMessage, OutgoingMessage, ReaderHandler,
        RegisterChannelResource, RegisterConnectionCallback, RegisterResponder, SplitConnection,
        WriterHandler,
    },
};

use super::{
    callbacks::ConnectionCallback,
    channel::{Channel, ChannelDispatcher},
    error::Error,
    security::SecurityCredentials,
    Result,
};

#[cfg(feature = "tls")]
use super::tls::TlsAdaptor;

#[cfg(feature = "compliance_assert")]
use crate::api::compliance_asserts::assert_path;

#[cfg(feature = "traces")]
use tracing::{debug, error, info};

#[cfg(feature = "urispec")]
use uriparse::URIReference;

const DEFAULT_AMQP_PORT: u16 = 5672;
const DEFAULT_AMQPS_PORT: u16 = 5671;
const DEFAULT_HEARTBEAT: u16 = 60;
const AMQP_SCHEME: &str = "amqp";
const AMQPS_SCHEME: &str = "amqps";

// per connection buffer
const OUTGOING_MESSAGE_BUFFER_SIZE: usize = 8192;
const CONNECTION_MANAGEMENT_COMMAND_BUFFER_SIZE: usize = 256;

const DEFAULT_LOCALE: &str = "en_US";

/////////////////////////////////////////////////////////////////////////////
/// Capabilities reported by the server when openning an connection.
///
/// It is part of [`ServerProperties`] reported from server.
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    consumer_cancel_notify: bool,
    publisher_confirms: bool,
    consumer_priorities: bool,
    authentication_failure_close: bool,
    per_consumer_qos: bool,
    connection_blocked: bool,
    exchange_exchange_bindings: bool,
    basic_nack: bool,
    direct_reply_to: bool,
}

impl ServerCapabilities {
    pub fn consumer_cancel_notify(&self) -> bool {
        self.consumer_cancel_notify
    }

    pub fn publisher_confirms(&self) -> bool {
        self.publisher_confirms
    }

    pub fn consumer_priorities(&self) -> bool {
        self.consumer_priorities
    }

    pub fn authentication_failure_close(&self) -> bool {
        self.authentication_failure_close
    }

    pub fn per_consumer_qos(&self) -> bool {
        self.per_consumer_qos
    }

    pub fn connection_blocked(&self) -> bool {
        self.connection_blocked
    }

    pub fn exchange_exchange_bindings(&self) -> bool {
        self.exchange_exchange_bindings
    }

    pub fn basic_nack(&self) -> bool {
        self.basic_nack
    }

    pub fn direct_reply_to(&self) -> bool {
        self.direct_reply_to
    }
}

/// Propertities reported by the server when openning an connection.
#[derive(Debug, Clone)]
pub struct ServerProperties {
    capabilities: ServerCapabilities,
    product: String,
    cluster_name: String,
    version: String,
}

impl ServerProperties {
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    pub fn cluster_name(&self) -> &str {
        self.cluster_name.as_ref()
    }

    pub fn version(&self) -> &str {
        self.version.as_ref()
    }

    pub fn product(&self) -> &str {
        self.product.as_ref()
    }
}

struct DropGuard {
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    is_open: Arc<AtomicBool>,
    connection_name: String,
}

impl DropGuard {
    fn new(
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        is_open: Arc<AtomicBool>,
        connection_name: String,
    ) -> Self {
        Self {
            outgoing_tx,
            is_open,
            connection_name,
        }
    }
}

/// Type represents an connection.
///
/// See documentation of each method.
/// See also documentation of [module][`self`] .
///
#[derive(Clone)]
pub struct Connection {
    shared: Arc<SharedConnectionInner>,
    /// `is_open` is not part of `shared` because DropGuard also
    /// need to access it.
    is_open: Arc<AtomicBool>,
    /// connection given to user has [Some] value,
    /// internal clones within library has [None] value,
    _guard: Option<Arc<DropGuard>>,
}

#[derive(Debug)]
struct SharedConnectionInner {
    server_properties: ServerProperties,
    connection_name: String,
    channel_max: ShortUint,
    frame_max: LongUint,
    heartbeat: ShortUint,
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
    shutdown_subscriber: broadcast::Sender<bool>,
}

/////////////////////////////////////////////////////////////////////////////
/// The arguments used by [`Connection::open`].
///
/// Methods can be chained in order to build the desired argument values, call
/// [`finish`] to finish chaining and returns a new argument.
///
/// Chaining configuration implies an additional clone when [`finish`] is called.
///
/// # Examples:
///
/// ## Chaining configuration style
///
/// ```
/// # use amqprs::security::SecurityCredentials;
/// # use amqprs::connection::OpenConnectionArguments;
///
/// // Create a default and update only `credentials` field, then return desired config.
/// let args = OpenConnectionArguments::default()
///     .credentials(SecurityCredentials::new_amqplain("user", "bitnami"))
///     .finish();
/// ```
///
/// ```
/// # use amqprs::security::SecurityCredentials;
/// # use amqprs::connection::OpenConnectionArguments;
///
/// // Create a new one and update the fields, then return desired config
/// let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
///     .virtual_host("myhost")
///     .connection_name("myconnection")
///     .finish();
/// ```
///
/// ## Non-chaining configuration style
///
/// ```
/// # use amqprs::security::SecurityCredentials;
/// # use amqprs::connection::OpenConnectionArguments;
///
/// // create a new and mutable argument
/// let mut args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");
/// // update fields of the mutable argument
/// args.virtual_host("myhost").connection_name("myconnection");
/// ```
///
/// ## Create from URI string if feature "urispec" is enabled
///
///
/// ```
/// # use amqprs::connection::OpenConnectionArguments;
/// # #[cfg(feature = "urispec")]
/// let args: OpenConnectionArguments = "amqp://user:bitnami@localhost".try_into().unwrap();
///
/// ```
///
/// [`Connection::open`]: struct.Connection.html#method.open
/// [`finish`]: struct.OpenConnectionArguments.html#method.finish

#[derive(Clone)]
pub struct OpenConnectionArguments {
    /// The server host. Default: "localhost".
    host: String,
    /// The server port. Default: 5672 by [AMQP 0-9-1 spec](https://www.rabbitmq.com/amqp-0-9-1-reference.html).
    port: u16,
    /// Default: "/". See [RabbitMQ vhosts](https://www.rabbitmq.com/vhosts.html).
    virtual_host: String,
    /// Default: [`None`], auto generate a connection name, otherwise use given connection name.
    connection_name: Option<String>,
    /// Default: use SASL/PLAIN authentication. See [RabbitMQ access control](https://www.rabbitmq.com/access-control.html#mechanisms).
    credentials: SecurityCredentials,
    /// Heartbeat timeout in seconds. See [RabbitMQ heartbeats](https://www.rabbitmq.com/heartbeats.html)
    /// Default: 60s.
    heartbeat: u16,
    /// scheme of URI for cross-checking consistency between provided scheme and TLS config
    /// If `amqps`scheme is used, TLS should be enabled and configured.
    scheme: Option<String>,
    /// SSL/TLS adaptor
    #[cfg(feature = "tls")]
    tls_adaptor: Option<TlsAdaptor>,
}

impl Default for OpenConnectionArguments {
    fn default() -> Self {
        Self {
            host: String::from("localhost"),
            port: DEFAULT_AMQP_PORT,
            virtual_host: String::from("/"),
            connection_name: None,
            credentials: SecurityCredentials::new_plain("guest", "guest"),
            heartbeat: 60,
            scheme: None,
            #[cfg(feature = "tls")]
            tls_adaptor: None,
        }
    }
}

impl OpenConnectionArguments {
    /// Return a new argument with default configuration.
    ///
    /// # Default
    ///
    /// Use virtual host "/", SASL/PLAIN authentication and auto generated connection name.
    ///
    pub fn new(host: &str, port: u16, username: &str, password: &str) -> Self {
        Self {
            host: host.to_owned(),
            port,
            virtual_host: String::from("/"),
            connection_name: None,
            credentials: SecurityCredentials::new_plain(username, password),
            heartbeat: 60,
            scheme: None,
            #[cfg(feature = "tls")]
            tls_adaptor: None,
        }
    }

    /// Set the host of the server.
    ///
    /// # Default
    ///
    /// "localhost"
    pub fn host(&mut self, host: &str) -> &mut Self {
        self.host = host.to_owned();
        self
    }

    /// Set the port of the server.
    ///
    /// # Default
    ///
    /// 5672 by [AMQP 0-9-1 spec](https://www.rabbitmq.com/amqp-0-9-1-reference.html).
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    /// Set the virtual host. See [RabbitMQ vhosts](https://www.rabbitmq.com/vhosts.html).
    ///
    /// # Default
    ///
    /// "/"
    pub fn virtual_host(&mut self, virtual_host: &str) -> &mut Self {
        #[cfg(feature = "compliance_assert")]
        assert_path(virtual_host);
        self.virtual_host = virtual_host.to_owned();
        self
    }

    /// Set the connection name.
    ///
    /// # Default
    ///
    /// Name is auto generated.
    pub fn connection_name(&mut self, connection_name: &str) -> &mut Self {
        self.connection_name = Some(connection_name.to_owned());
        self
    }

    /// Set the user credentials. See [RabbitMQ access control](https://www.rabbitmq.com/access-control.html#mechanisms).
    ///
    /// # Default
    ///
    /// SASL/PLAIN authentication, "guest" as both username and password.
    pub fn credentials(&mut self, credentials: SecurityCredentials) -> &mut Self {
        self.credentials = credentials;
        self
    }
    /// Set the heartbeat timeout in seconds. See [RabbitMQ heartbeats](https://www.rabbitmq.com/heartbeats.html).
    ///
    /// # Default
    ///
    /// 60 seconds.
    pub fn heartbeat(&mut self, heartbeat: u16) -> &mut Self {
        self.heartbeat = heartbeat;
        self
    }

    /// Set SSL/TLS adaptor. Set to enable SSL/TLS connection.
    ///
    /// # Default
    ///
    /// No SSL/TLS enabled
    #[cfg(feature = "tls")]
    pub fn tls_adaptor(&mut self, tls_adaptor: TlsAdaptor) -> &mut Self {
        self.tls_adaptor = Some(tls_adaptor);
        self
    }

    /// Finish chaining and returns a new argument according to chained configurations.
    ///
    /// It actually clones the resulted configurations.
    ///
    pub fn finish(&mut self) -> Self {
        self.clone()
    }
}

#[cfg(feature = "urispec")]
impl TryFrom<&str> for OpenConnectionArguments {
    type Error = Error;

    /// Create a new OpenConnectionArguments from a URI &str. Mostly follows the [AMQP URI spec](https://www.rabbitmq.com/uri-spec.html). See below for exceptions.
    ///
    /// If the URI is invalid, a UriError error is returned.
    ///
    /// If no port is specified, the default port of 5672 is used (as per the spec).
    /// If no host is speecified or is zero-length, a UriError error is returned. Note that this is different from the spec, which allows for EmptyHost. This is because the host is used to create a TCP connection, and an empty host is invalid.
    ///
    fn try_from(uri: &str) -> Result<Self> {
        let pu = URIReference::try_from(uri)?;

        // Check scheme
        let scheme = pu
            .scheme()
            .ok_or_else(|| Error::UriError(String::from("No URI scheme")))?
            .as_str();

        // Set the default port depending on the scheme.
        let default_port: u16 = match scheme {
            AMQP_SCHEME => DEFAULT_AMQP_PORT,
            AMQPS_SCHEME => {
                if cfg!(feature = "tls") {
                    DEFAULT_AMQPS_PORT
                } else {
                    return Err(Error::UriError(format!(
                        "TLS feature should be enabled to use scheme: {}",
                        scheme
                    )));
                }
            }
            _ => {
                return Err(Error::UriError(format!(
                    "Unsupported URI scheme: {}",
                    scheme
                )))
            }
        };

        // Check authority
        let pu_authority = pu
            .authority()
            .ok_or_else(|| Error::UriError(String::from("Invalid URI authority")))?;
        let pu_authority_username = pu_authority
            .username()
            .map(|v| v.as_str())
            .unwrap_or("guest");
        let pu_authority_password = pu_authority
            .password()
            .map(|v| v.as_str())
            .unwrap_or("guest");

        // Apply authority
        let host = pu_authority.host().to_string();
        let mut args = OpenConnectionArguments::new(
            host.as_str(),
            pu_authority.port().unwrap_or(default_port),
            pu_authority_username,
            pu_authority_password,
        );
        args.scheme = Some(scheme.to_owned());

        // Check & apply virtual host
        let pu_path = pu.path().to_string();
        if pu_path.len() <= 1 {
            args.virtual_host("/");
        } else {
            args.virtual_host(&pu_path[1..]);
        }

        // Check & apply query
        let pu_q = pu.query().map(|v| v.as_str()).ok_or(|| "").unwrap_or("");

        // Return early if there is no query since all that is left is to process the query
        if pu_q.is_empty() {
            return Ok(args);
        }

        // Create a hash map for query
        // TODO: This map needs to be of type (or similar) <&str, Vec<&str>> to support multiple values for the same key, which is both possible and plausible in the URI spec
        // This is being left as a TODO because there is a bit of research to do in order to determine what actions when multiple value are provided.
        let pu_q_map: std::collections::HashMap<&str, &str> = pu_q
            .split('&')
            .map(|s| {
                let mut split = s.split('=');
                let key = split.next().unwrap();
                let value = split.next().unwrap();
                (key, value)
            })
            .collect();

        // Apply heartbeat
        let heartbeat = pu_q_map
            .get("heartbeat")
            .map(|v| v.parse::<u16>().unwrap_or(DEFAULT_HEARTBEAT))
            .unwrap_or(DEFAULT_HEARTBEAT);
        args.heartbeat(heartbeat);

        if scheme == AMQPS_SCHEME {
            #[cfg(feature = "tls")]
            args.tls_adaptor(
                TlsAdaptor::without_client_auth(None, host.to_string())
                    .map_err(|e| Error::UriError(format!("error creating TLS adaptor: {}", e)))?,
            );

            #[cfg(not(feature = "tls"))]
            return Err(Error::UriError("can't create amqps url without the `tls` feature enabled".to_string()));
        }

        Ok(args)
    }
}

/////////////////////////////////////////////////////////////////////////////

impl Connection {
    /// Open and returns a new connection.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if any step goes wrong during openning an connection.
    pub async fn open(args: &OpenConnectionArguments) -> Result<Self> {
        #[cfg(feature = "tls")]
        let mut io_conn = match &args.tls_adaptor {
            Some(tls_adaptor) => {
                if let Some(scheme) = &args.scheme {
                    if scheme == AMQP_SCHEME {
                        return Err(Error::UriError(format!(
                            "Try to open a secure connection with '{}' scheme",
                            scheme
                        )));
                    }
                }
                SplitConnection::open_tls(
                    &format!("{}:{}", args.host, args.port),
                    &tls_adaptor.domain,
                    &tls_adaptor.connector,
                )
                .await?
            }

            None => {
                if let Some(scheme) = &args.scheme {
                    if scheme == AMQPS_SCHEME {
                        return Err(Error::UriError(format!(
                            "Try to open a regular connection with '{}' scheme",
                            scheme
                        )));
                    }
                }
                SplitConnection::open(&format!("{}:{}", args.host, args.port)).await?
            }
        };
        #[cfg(not(feature = "tls"))]
        let mut io_conn = {
            if let Some(scheme) = &args.scheme {
                if scheme == AMQPS_SCHEME {
                    return Err(Error::UriError(format!(
                        "Try to open a regular connection with '{}' scheme",
                        scheme
                    )));
                }
            }
            SplitConnection::open(&format!("{}:{}", args.host, args.port)).await?
        };

        // C:protocol-header
        Self::negotiate_protocol(&mut io_conn).await?;

        // if no given connection name, generate one
        let connection_name = match args.connection_name {
            Some(ref given_name) => given_name.clone(),
            None => generate_connection_name(&format!(
                "{}:{}{}",
                args.host, args.port, args.virtual_host
            )),
        };
        // construct client properties
        let mut client_properties = AmqpPeerProperties::new();
        client_properties.insert(
            "connection_name".try_into().unwrap(),
            FieldValue::S(connection_name.clone().try_into().unwrap()),
        );
        // fields required by spec: "product", "platform", "version"
        client_properties.insert(
            "product".try_into().unwrap(),
            FieldValue::S("AMQPRS".try_into().unwrap()),
        );
        client_properties.insert(
            "platform".try_into().unwrap(),
            FieldValue::S("Rust".try_into().unwrap()),
        );
        client_properties.insert(
            "version".try_into().unwrap(),
            FieldValue::S("0.1".try_into().unwrap()),
        );
        let mut client_properties_capabilities = FieldTable::new();
        client_properties_capabilities
            .insert("consumer_cancel_notify".try_into().unwrap(), true.into());
        client_properties.insert(
            "capabilities".try_into().unwrap(),
            FieldValue::F(client_properties_capabilities),
        );

        // S: `Start` C: `StartOk`
        let server_properties =
            Self::start_connection_negotiation(&mut io_conn, client_properties, args).await?;

        // S: 'Tune' C: `TuneOk`
        let (channel_max, frame_max, heartbeat) =
            Self::tuning_parameters(&mut io_conn, args.heartbeat).await?;
        // C: Open
        let open = Open::new(
            args.virtual_host.clone().try_into().unwrap(),
            "".try_into().unwrap(),
        )
        .into_frame();
        io_conn
            .write_frame(DEFAULT_CONN_CHANNEL, open, FRAME_MIN_SIZE)
            .await?;

        // S: OpenOk
        let (_, frame) = io_conn.read_frame().await?;
        unwrap_expected_method!(
            frame,
            Frame::OpenOk,
            Error::ConnectionOpenError(format!("failed to open connection, reason: {}", frame))
        )?;

        // spawn network management tasks and get internal channel' sender half.
        let (outgoing_tx, outgoing_rx) = mpsc::channel(OUTGOING_MESSAGE_BUFFER_SIZE);
        let (conn_mgmt_tx, conn_mgmt_rx) = mpsc::channel(CONNECTION_MANAGEMENT_COMMAND_BUFFER_SIZE);
        let (shutdown_notifer, _) = broadcast::channel::<bool>(1);
        let shared = Arc::new(SharedConnectionInner {
            server_properties,
            connection_name,
            channel_max,
            frame_max,
            heartbeat,
            outgoing_tx,
            conn_mgmt_tx,
            shutdown_subscriber: shutdown_notifer.clone(),
        });

        // open state of connection
        let is_open = Arc::new(AtomicBool::new(true));

        let _guard = Some(Arc::new(DropGuard::new(
            shared.outgoing_tx.clone(),
            is_open.clone(),
            shared.connection_name.clone(),
        )));
        let new_amqp_conn = Self {
            shared,
            is_open,
            _guard,
        };

        // spawn handlers for reader and writer of network connection
        new_amqp_conn
            .spawn_handlers(
                io_conn,
                outgoing_rx,
                conn_mgmt_rx,
                heartbeat,
                shutdown_notifer,
            )
            .await;

        // register channel resource for connection's default channel
        new_amqp_conn
            .register_channel_resource(Some(DEFAULT_CONN_CHANNEL), ChannelResource::new(None))
            .await
            .ok_or_else(|| {
                Error::ConnectionOpenError("failed to register channel resource".to_string())
            })?;
        #[cfg(feature = "traces")]
        info!("open connection {}", new_amqp_conn.connection_name());
        Ok(new_amqp_conn)
    }

    /// Protocol negotiation according to AMQP 0-9-1
    ///
    /// Only support AMQP 0-9-1.
    ///
    /// # Errors
    ///
    /// Returns error if fail to send protocol header.
    async fn negotiate_protocol(io_conn: &mut SplitConnection) -> Result<()> {
        // only support AMQP 0-9-1 at present
        io_conn.write(&ProtocolHeader::default()).await?;
        Ok(())
    }

    /// Start connection negotiation according to AMQP 0-9-1 methods Start/StartOk
    ///
    /// Returns
    ///
    /// [`ServerProperties`]
    ///
    /// # Errors
    ///
    /// Returns error when encounters any protocol error.
    async fn start_connection_negotiation(
        io_conn: &mut SplitConnection,
        client_properties: AmqpPeerProperties,
        args: &OpenConnectionArguments,
    ) -> Result<ServerProperties> {
        // S: 'Start'
        let (_, frame) = io_conn.read_frame().await?;
        let mut start = unwrap_expected_method!(
            frame,
            Frame::Start,
            Error::ConnectionOpenError(format!(
                "failed to negotiate connection params, reason: {}",
                frame
            ))
        )?;
        // get server supported locales
        if !start
            .locales
            .as_ref()
            .split(' ')
            .any(|v| DEFAULT_LOCALE == v)
        {
            return Err(Error::ConnectionOpenError(format!(
                "locale '{}' is not supported by server",
                DEFAULT_LOCALE
            )));
        }
        // get server supported authentication mechanisms
        if !start
            .mechanisms
            .as_ref()
            .split(' ')
            .any(|v| args.credentials.get_mechanism_name() == v)
        {
            return Err(Error::ConnectionOpenError(format!(
                "authentication '{}' is not supported by server",
                args.credentials.get_mechanism_name()
            )));
        }

        // get server capabilities
        let mut caps_table: FieldTable = start
            .server_properties
            .remove(&"capabilities".try_into().unwrap())
            .unwrap_or_else(|| FieldValue::F(FieldTable::default()))
            .try_into()
            .unwrap();
        // helper closure to get bool FieldValue
        let mut unwrap_bool_field = |key: &str| {
            let value: bool = caps_table
                .remove(&key.try_into().unwrap())
                .unwrap_or(FieldValue::t(false))
                .try_into()
                .unwrap();
            value
        };

        let capabilities = ServerCapabilities {
            consumer_cancel_notify: unwrap_bool_field("consumer_cancel_notify"),
            publisher_confirms: unwrap_bool_field("publisher_confirms"),
            consumer_priorities: unwrap_bool_field("consumer_priorities"),
            authentication_failure_close: unwrap_bool_field("authentication_failure_close"),
            per_consumer_qos: unwrap_bool_field("per_consumer_qos"),
            connection_blocked: unwrap_bool_field("connection.blocked"),
            exchange_exchange_bindings: unwrap_bool_field("exchange_exchange_bindings"),
            basic_nack: unwrap_bool_field("basic.nack"),
            direct_reply_to: unwrap_bool_field("direct_reply_to"),
        };

        // helper closure to get LongStr FieldValue
        let mut unwrap_longstr_field = |key: &str| {
            let value: LongStr = start
                .server_properties
                .remove(&key.try_into().unwrap())
                .unwrap_or_else(|| FieldValue::S("unknown".try_into().unwrap()))
                .try_into()
                .unwrap();
            value
        };

        let server_properties = ServerProperties {
            capabilities,
            product: unwrap_longstr_field("product").into(),
            cluster_name: unwrap_longstr_field("cluster_name").into(),
            version: unwrap_longstr_field("version").into(),
        };

        // C: 'StartOk'
        let resopnse = args.credentials.get_response().try_into().unwrap();
        // TODO: support different machanisms: PLAIN, AMQPLAIN, SSL
        // TODO: handle locale selection
        let start_ok = StartOk::new(
            client_properties,
            args.credentials.get_mechanism_name().try_into().unwrap(),
            resopnse,
            DEFAULT_LOCALE.try_into().unwrap(),
        );

        io_conn
            .write_frame(DEFAULT_CONN_CHANNEL, start_ok.into_frame(), FRAME_MIN_SIZE)
            .await?;
        Ok(server_properties)
    }

    /// Tuning for channel_max, frame_max, heartbeat between client and server.
    ///
    /// # Returns
    ///
    ///  `(channel_max, frame_max, heartbeat)`
    async fn tuning_parameters(
        io_conn: &mut SplitConnection,
        heartbeat: ShortUint,
    ) -> Result<(ShortUint, LongUint, ShortUint)> {
        // S: 'Tune'
        let (_, frame) = io_conn.read_frame().await?;
        let tune = unwrap_expected_method!(
            frame,
            Frame::Tune,
            Error::ConnectionOpenError(format!(
                "failed to tune connection params, reason: {}",
                frame
            ))
        )?;

        // according to https://www.rabbitmq.com/heartbeats.html
        let new_heartbeat = if tune.heartbeat() == 0 || heartbeat == 0 {
            std::cmp::max(tune.heartbeat(), heartbeat)
        } else {
            std::cmp::min(tune.heartbeat(), heartbeat)
        };

        // No tunning of channel_max and frame_max
        #[cfg(feature = "compliance_assert")]
        {
            assert_ne!(0, tune.channel_max());
            assert!(tune.frame_max() >= FRAME_MIN_SIZE);
        }
        // just accept the values from server
        let new_channel_max = tune.channel_max();
        let new_frame_max = tune.frame_max();

        // C: TuneOk
        let tune_ok = TuneOk::new(new_channel_max, new_frame_max, new_heartbeat);

        io_conn
            .write_frame(DEFAULT_CONN_CHANNEL, tune_ok.into_frame(), FRAME_MIN_SIZE)
            .await?;
        Ok((new_channel_max, new_frame_max, new_heartbeat))
    }

    /// Get connection name.
    pub fn connection_name(&self) -> &str {
        &self.shared.connection_name
    }

    /// Get the maximum total number of channels of the connection.
    pub fn channel_max(&self) -> u16 {
        self.shared.channel_max
    }
    /// Get The largest frame size that the client and server will use for the connection.
    pub fn frame_max(&self) -> u32 {
        self.shared.frame_max
    }
    /// Get the server propertities reported by server.
    pub fn server_properties(&self) -> &ServerProperties {
        &self.shared.server_properties
    }
    async fn register_responder(
        &self,
        channel_id: AmqpChannelId,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<IncomingMessage>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterResponder {
            channel_id,
            method_header,
            responder,
            acker,
        };
        self.shared
            .conn_mgmt_tx
            .send(ConnManagementCommand::RegisterResponder(cmd))
            .await?;
        acker_rx.await?;
        Ok(responder_rx)
    }

    /// Register callbacks for handling asynchronous message from server for the connection.
    ///
    /// User should always register callbacks. See [`callbacks`] documentation.
    ///
    /// # Errors
    ///
    /// Returns error if fail to send registration command.
    /// If returns [`Err`], user can try again until registration succeed.
    ///
    /// [`callbacks`]: ../callbacks/index.html
    pub async fn register_callback<F>(&self, callback: F) -> Result<()>
    where
        F: ConnectionCallback + Send + 'static,
    {
        let cmd = RegisterConnectionCallback {
            callback: Box::new(callback),
        };
        self.shared
            .conn_mgmt_tx
            .send(ConnManagementCommand::RegisterConnectionCallback(cmd))
            .await?;
        Ok(())
    }

    pub(crate) fn set_is_open(&self, is_open: bool) {
        self.is_open.store(is_open, Ordering::Relaxed);
    }

    /// Returns `true` if connection is open.
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }

    /// Returns interval of heartbeat in seconds.
    pub fn heartbeat(&self) -> u16 {
        self.shared.heartbeat
    }

    pub(crate) async fn register_channel_resource(
        &self,
        channel_id: Option<AmqpChannelId>,
        resource: ChannelResource,
    ) -> Option<AmqpChannelId> {
        let (acker, acker_rx) = oneshot::channel();
        let cmd = ConnManagementCommand::RegisterChannelResource(RegisterChannelResource {
            channel_id,
            resource,
            acker,
        });

        // If no channel id is given, it will be allocated by management task and included in acker response
        // otherwise same id will be received in response
        if let Err(err) = self.shared.conn_mgmt_tx.send(cmd).await {
            #[cfg(feature = "traces")]
            debug!(
                "failed to register channel resource on connection {}, cause: {}",
                self, err
            );
            return None;
        }

        // expect a channel id in response
        match acker_rx.await {
            Ok(res) => {
                if res.is_none() {
                    #[cfg(feature = "traces")]
                    debug!(
                        "failed to allocate/reserve channel id on connection {}",
                        self
                    );
                }
                res
            }
            Err(err) => {
                #[cfg(feature = "traces")]
                debug!(
                    "failed to register channel resource on connection {}, cause: {}",
                    self, err
                );
                None
            }
        }
    }

    /// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
    pub(crate) async fn spawn_handlers(
        &self,
        io_conn: SplitConnection,
        outgoing_rx: mpsc::Receiver<OutgoingMessage>,
        conn_mgmt_rx: mpsc::Receiver<ConnManagementCommand>,
        heartbeat: ShortUint,
        shutdown_notifer: broadcast::Sender<bool>,
    ) {
        // Spawn two tasks for the connection
        // - one task for writer
        // - one task for reader
        let (reader, writer) = io_conn.into_split();

        // spawn task for write connection handler
        let wh = WriterHandler::new(
            writer,
            outgoing_rx,
            shutdown_notifer.subscribe(),
            self.clone_no_drop_guard(),
        );
        tokio::spawn(async move {
            wh.run_until_shutdown(heartbeat).await;
        });
        // spawn task for read connection handler
        let rh = ReaderHandler::new(
            reader,
            self.clone_no_drop_guard(),
            self.shared.outgoing_tx.clone(),
            conn_mgmt_rx,
            self.shared.channel_max,
            shutdown_notifer,
        );
        tokio::spawn(async move {
            rh.run_until_shutdown(heartbeat).await;
        });
    }

    /// Open and return a new AMQP channel.
    ///
    /// `channel_id` range: 1 to 65535.
    ///
    /// Automatically generate an id if input `channel_id` = [`None`],
    /// otherwise, use the given input id.
    ///
    /// # Errors
    ///
    /// Returns error if the given `channel_id` is occupied, or any failure
    /// in resource allocation and communication with server.
    pub async fn open_channel(&self, channel_id: Option<AmqpChannelId>) -> Result<Channel> {
        // channel id 0 can't be used, it is reserved for connection
        assert_ne!(Some(DEFAULT_CONN_CHANNEL), channel_id);

        let (dispatcher_tx, dispatcher_rx) = mpsc::unbounded_channel();
        let (dispatcher_mgmt_tx, dispatcher_mgmt_rx) = mpsc::unbounded_channel();

        // acquire the channel id to be used to open channel
        let channel_id = self
            .register_channel_resource(channel_id, ChannelResource::new(Some(dispatcher_tx)))
            .await
            .ok_or_else(|| {
                Error::ChannelOpenError("failed to register channel resource".to_string())
            })?;

        // register responder, use the acquired channel id
        let responder_rx = self
            .register_responder(channel_id, OpenChannelOk::header())
            .await?;

        synchronous_request!(
            self.shared.outgoing_tx,
            (channel_id, OpenChannel::new().into_frame()),
            responder_rx,
            Frame::OpenChannelOk,
            Error::ChannelOpenError
        )?;

        // create channel instance
        // set default prefetch count to 10
        let channel = Channel::new(
            AtomicBool::new(true),
            self.clone_no_drop_guard(),
            channel_id,
            self.shared.outgoing_tx.clone(),
            self.shared.conn_mgmt_tx.clone(),
            dispatcher_mgmt_tx,
        );

        let dispatcher = ChannelDispatcher::new(
            channel.clone_as_secondary(),
            dispatcher_rx,
            dispatcher_mgmt_rx,
        );
        dispatcher.spawn().await;
        #[cfg(feature = "traces")]
        info!("open channel {}", channel);

        Ok(channel)
    }

    /// This method notify server that the connection has been blocked and does not
    /// accept new publishes.
    ///
    /// # Errors
    ///
    /// Returns error if fails to send indication to server.
    pub async fn blocked(&self, reason: &str) -> Result<()> {
        let blocked = Blocked::new(reason.to_owned().try_into().unwrap());

        self.shared
            .outgoing_tx
            .send((DEFAULT_CONN_CHANNEL, blocked.into_frame()))
            .await?;
        Ok(())
    }

    /// This method notify server that the connection has been unblocked and does not
    /// accept new publishes.
    ///
    /// # Errors
    ///
    /// Returns error if fails to send indication to server.
    pub async fn unblocked(&self) -> Result<()> {
        let unblocked = Unblocked;

        self.shared
            .outgoing_tx
            .send((DEFAULT_CONN_CHANNEL, unblocked.into_frame()))
            .await?;
        Ok(())
    }

    /// Send request to server to close the connection.
    ///
    /// To gracefully shutdown the connection, recommended to `close` the
    /// connection explicitly instead of relying on `drop`.
    ///
    /// This method consume the connection, so even it may return error,
    /// connection will anyway be dropped.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    pub async fn close(self) -> Result<()> {
        if let Ok(true) =
            self.is_open
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            #[cfg(feature = "traces")]
            info!("close connection {}", self);
            self.close_handshake().await?;
        }
        Ok(())
    }

    async fn close_handshake(&self) -> Result<()> {
        // connection's close method , should use default channel id
        let responder_rx = self
            .register_responder(DEFAULT_CONN_CHANNEL, CloseOk::header())
            .await
            .map_err(|err| {
                Error::ConnectionCloseError(format!("failed to register responder {}", err))
            })?;

        let close = Close::default();
        synchronous_request!(
            self.shared.outgoing_tx,
            (DEFAULT_CONN_CHANNEL, close.into_frame()),
            responder_rx,
            Frame::CloseOk,
            Error::ConnectionCloseError
        )?;

        Ok(())
    }

    pub(crate) fn clone_no_drop_guard(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            is_open: self.is_open.clone(),
            _guard: None,
        }
    }

    /// Wait until the underlying network I/O failure occurs.
    ///
    /// It will block the current async task. To handle it asynchronously,
    /// use `tokio::spawn` to run it in a seperate task.
    ///
    /// # Returns
    ///
    /// Return `true` if got notification due to network I/O failure, otherwise return `false`.
    ///
    pub async fn listen_network_io_failure(&self) -> bool {
        let mut shutdown_listener = self.shared.shutdown_subscriber.subscribe();
        (shutdown_listener.recv().await).unwrap_or(false)
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        if let Ok(true) =
            self.is_open
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            let connection_name = self.connection_name.clone();
            let outgoing_tx = self.outgoing_tx.clone();
            tokio::spawn(async move {
                #[cfg(feature = "traces")]
                info!("try to close connection {} at drop", connection_name);

                let close = Close::default();

                if let Err(err) = outgoing_tx
                    .send((DEFAULT_CONN_CHANNEL, close.into_frame()))
                    .await
                {
                    #[cfg(feature = "traces")]
                    error!(
                        "failed to gracefully close connection {} at drop, cause: '{}'",
                        connection_name, err
                    );
                }
            });
        }
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "'{} [{}]'",
            self.connection_name(),
            if self.is_open() { "open" } else { "closed" }
        )
    }
}
/// In reality, one client can't open `usize::MAX` connections :)
/// Use simple algorithm to generate large enough number of unique names,
/// to avoid using any external crate.
fn generate_connection_name(domain: &str) -> String {
    const PREFIX: &str = "AMQPRS";
    // global counter, gives `usize::MAX` unique values
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    // construct a name
    format!(
        "{}{:03}@{}",
        PREFIX,
        COUNTER.fetch_add(1, Ordering::Relaxed),
        domain
    )
}

/////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::{generate_connection_name, Connection, OpenConnectionArguments};
    use crate::security::SecurityCredentials;
    use crate::test_utils::setup_logging;
    use std::{collections::HashSet, thread};
    use tokio::time;

    #[tokio::test]
    async fn test_channel_open_close() {
        setup_logging();
        {
            // test close on drop
            let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

            let connection = Connection::open(&args).await.unwrap();
            {
                // test close on drop
                let _channel = connection.open_channel(None).await.unwrap();
            }
            time::sleep(time::Duration::from_millis(100)).await;
        }
        // wait for finished, otherwise runtime exit before all tasks are done
        time::sleep(time::Duration::from_millis(100)).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_multi_conn_open_close() {
        setup_logging();

        let mut handles = vec![];

        for _ in 0..10 {
            let handle = tokio::spawn(async {
                let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

                time::sleep(time::Duration::from_millis(200)).await;
                let connection = Connection::open(&args).await.unwrap();
                time::sleep(time::Duration::from_millis(200)).await;
                connection.close().await.unwrap();
            });
            handles.push(handle);
        }
        for h in handles {
            h.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_connection_clone_and_drop() {
        setup_logging();
        {
            // test close on drop
            let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

            let conn1 = Connection::open(&args).await.unwrap();
            let conn2 = conn1.clone();
            tokio::spawn(async move {
                assert!(conn2.is_open());
            });
            assert!(conn1.is_open());
        }
        // wait for finished, otherwise runtime exit before all tasks are done
        time::sleep(time::Duration::from_millis(100)).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_multi_channel_open_close() {
        setup_logging();
        {
            let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
                .connection_name("test_multi_channel_open_close")
                .finish();

            let connection = Connection::open(&args).await.unwrap();
            let mut handles = vec![];
            let num_loop = 10;
            for _ in 0..num_loop {
                let ch = connection.open_channel(None).await.unwrap();
                let handle = tokio::spawn(async move {
                    let _ch = ch;
                    time::sleep(time::Duration::from_millis(100)).await;
                });
                handles.push(handle);
            }
            for h in handles {
                h.await.unwrap();
            }
            // wait for all channel tasks done
            time::sleep(time::Duration::from_millis(100 * num_loop)).await;
            // now connection drop
        }
        // wait for connection close task done.
        time::sleep(time::Duration::from_millis(100)).await;
    }

    #[test]
    fn test_name_generation() {
        let n = 100;
        let mut jh = Vec::with_capacity(n);
        let mut res = HashSet::with_capacity(n);
        for _ in 0..n {
            jh.push(thread::spawn(|| generate_connection_name("testdomain")));
        }
        for h in jh {
            assert!(res.insert(h.join().unwrap()));
        }
    }

    #[tokio::test]
    async fn test_duplicated_conn_name_is_accpeted_by_server() {
        setup_logging();

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
            .connection_name("amq.cname-test")
            .finish();

        let conn1 = Connection::open(&args).await.unwrap();
        let conn2 = Connection::open(&args).await.unwrap();
        time::sleep(time::Duration::from_millis(100)).await;
        conn1.close().await.unwrap();
        conn2.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_auth_amqplain() {
        setup_logging();

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
            .credentials(SecurityCredentials::new_amqplain("user", "bitnami"))
            .finish();
        Connection::open(&args).await.unwrap();
    }

    #[tokio::test]
    async fn test_block_unblock() {
        setup_logging();

        let args = OpenConnectionArguments::default()
            .credentials(SecurityCredentials::new_plain("user", "bitnami"))
            .finish();
        let conn = Connection::open(&args).await.unwrap();
        conn.blocked("test blocked").await.unwrap();
        conn.unblocked().await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "failed to register channel resource")]
    async fn test_open_already_opened_channel() {
        setup_logging();

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami")
            .credentials(SecurityCredentials::new_amqplain("user", "bitnami"))
            .finish();
        let connection = Connection::open(&args).await.unwrap();
        let id = Some(9);
        let _ch1 = connection.open_channel(id).await.unwrap();
        let _ch2 = connection.open_channel(id).await.unwrap();
    }

    #[cfg(feature = "urispec")]
    #[test]
    fn test_openconnectionarguments_try_from() {
        let args = OpenConnectionArguments::try_from("amqp://user:pass@host:10000/vhost").unwrap();
        assert_eq!(args.host, "host");
        assert_eq!(args.port, 10000);
        assert_eq!(args.virtual_host, "vhost");

        let args =
            OpenConnectionArguments::try_from("amqp://user%61:%61pass@ho%61st:10000/v%2fhost")
                .unwrap();
        assert_eq!(args.host, "ho%61st");
        assert_eq!(args.port, 10000);
        assert_eq!(args.virtual_host, "v%2fhost");

        let args = OpenConnectionArguments::try_from("amqp://").unwrap();
        assert_eq!(args.host, "");
        assert_eq!(args.port, 5672);
        assert_eq!(args.virtual_host, "/");

        let args = OpenConnectionArguments::try_from("amqp://:@/").unwrap();
        assert_eq!(args.host, "");
        assert_eq!(args.port, 5672);
        assert_eq!(args.virtual_host, "/");

        let args = OpenConnectionArguments::try_from("amqp://user@").unwrap();
        assert_eq!(args.host, "");
        assert_eq!(args.port, 5672);
        assert_eq!(args.virtual_host, "/");

        let args = OpenConnectionArguments::try_from("amqp://user:pass@").unwrap();
        assert_eq!(args.host, "");
        assert_eq!(args.port, 5672);
        assert_eq!(args.virtual_host, "/");

        let args = OpenConnectionArguments::try_from("amqp://host").unwrap();
        assert_eq!(args.host, "host");
        assert_eq!(args.port, 5672);
        assert_eq!(args.virtual_host, "/");

        let args = OpenConnectionArguments::try_from("amqp://:10000").unwrap();
        assert_eq!(args.host, "");
        assert_eq!(args.port, 10000);
        assert_eq!(args.virtual_host, "/");

        let args = OpenConnectionArguments::try_from("amqp://host:10000").unwrap();
        assert_eq!(args.host, "host");
        assert_eq!(args.port, 10000);
        assert_eq!(args.virtual_host, "/");

        // Results in an error because of invalid URI scheme
        let args = OpenConnectionArguments::try_from("fsdkfjflsd::/fsdfsdfsd:sfsd/");
        assert!(args.is_err());

        // Results in an error because of Completely invalid URI
        let args = OpenConnectionArguments::try_from("fsdkfjflsd/fsdfsdfsdsfsd/");
        assert!(args.is_err());

        let args = OpenConnectionArguments::try_from("amqp://[::1]").unwrap();
        assert_eq!(args.host, "[::1]");
        assert_eq!(args.port, 5672);
        assert_eq!(args.virtual_host, "/");
        assert_eq!(args.heartbeat, 60);

        let args = OpenConnectionArguments::try_from("amqp://[::1]?heartbeat=30").unwrap();
        assert_eq!(args.host, "[::1]");
        assert_eq!(args.port, 5672);
        assert_eq!(args.virtual_host, "/");
        assert_eq!(args.heartbeat, 30);
    }

    #[cfg(all(feature = "urispec", not(feature = "tls")))]
    #[test]
    fn test_urispec_amqps_without_tls() {
        match OpenConnectionArguments::try_from("amqps://user:bitnami@localhost?heartbeat=10") {
            Ok(_) => panic!("Unexpected ok"),
            Err(e) => assert!(matches!(e, crate::api::Error::UriError(_))),
        }
    }

    #[cfg(all(feature = "urispec", feature = "tls"))]
    #[test]
    fn test_urispec_amqps() {
        let args = OpenConnectionArguments::try_from("amqps://user:bitnami@localhost?heartbeat=10")
            .unwrap();
        assert_eq!(args.host, "localhost");
        assert_eq!(args.port, 5671);
        assert_eq!(args.virtual_host, "/");
        assert_eq!(args.heartbeat, 10);
        let tls_adaptor = args.tls_adaptor.unwrap();
        assert_eq!(tls_adaptor.domain, "localhost");
    }

    #[cfg(all(feature = "urispec", feature = "tls"))]
    #[tokio::test]
    #[should_panic(expected = "UriError")]
    async fn test_amqp_scheme_with_tls() {
        ////////////////////////////////////////////////////////////////
        // TLS specific configuration
        let current_dir = std::env::current_dir().unwrap();
        let current_dir = current_dir.join("../rabbitmq_conf/client/");

        let root_ca_cert = current_dir.join("ca_certificate.pem");
        let client_cert = current_dir.join("client_AMQPRS_TEST_certificate.pem");
        let client_private_key = current_dir.join("client_AMQPRS_TEST_key.pem");
        // domain should match the certificate/key files
        let domain = "AMQPRS_TEST";
        let tls_adaptor = crate::tls::TlsAdaptor::with_client_auth(
            Some(root_ca_cert.as_path()),
            client_cert.as_path(),
            client_private_key.as_path(),
            domain.to_owned(),
        )
        .unwrap();
        let args = OpenConnectionArguments::try_from("amqp://user:bitnami@localhost?heartbeat=10")
            .unwrap()
            .tls_adaptor(tls_adaptor)
            .finish();
        Connection::open(&args).await.unwrap();
    }
}
