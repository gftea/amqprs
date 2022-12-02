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
//! let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");
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
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use amqp_serde::types::{
    AmqpChannelId, AmqpPeerProperties, FieldTable, FieldValue, LongStr, LongUint, ShortUint,
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{debug, error, info, trace};

use crate::{
    channel,
    frame::{
        Blocked, Close, CloseOk, Frame, MethodHeader, Open, OpenChannel, OpenChannelOk,
        ProtocolHeader, StartOk, TuneOk, Unblocked, DEFAULT_CONN_CHANNEL,
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

//  TODO: move below constants gto be part of static configuration of connection
const DISPATCHER_MESSAGE_BUFFER_SIZE: usize = 256;
const DISPATCHER_MANAGEMENT_COMMAND_BUFFER_SIZE: usize = 64;
const OUTGOING_MESSAGE_BUFFER_SIZE: usize = 256;
const CONNECTION_MANAGEMENT_COMMAND_BUFFER_SIZE: usize = 64;

const DEFAULT_LOCALE: &str = "en_US";

/////////////////////////////////////////////////////////////////////////////
/// Capabilities reported by the server when openning an AMQP connection.
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

/// Propertities reported by the server when openning an AMQP connection.
#[derive(Debug, Clone)]
pub struct ServerProperties {
    capabilities: ServerCapabilities,
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
}

/// Type represents an AMQP connection.
///
/// See documentation of each method.
/// See also documentation of [module][`self`] .
///
#[derive(Debug, Clone)]
pub struct Connection {
    shared: Arc<SharedConnectionInner>,
}

#[derive(Debug)]
struct SharedConnectionInner {
    server_properties: ServerProperties,
    connection_name: String,
    channel_max: ShortUint,
    frame_max: LongUint,
    heartbeat: ShortUint,
    is_open: AtomicBool,
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
}

/////////////////////////////////////////////////////////////////////////////
/// The arguments used by [`Connection::open`].
///
/// Methods can be chained in order to build the desired argument values, call
/// [`finish`] to finish chaining and returns a new argument.
///
/// # Examples:
/// ```
/// # use amqprs::security::SecurityCredentials;
/// # use amqprs::connection::OpenConnectionArguments;
/// // Update `credentials` field, leaving remaining fields as default value.
/// let args = OpenConnectionArguments::default()
///     .credentials(SecurityCredentials::new_amqplain("user", "bitnami"))
///     .finish();
/// ```
///
/// ```
/// # use amqprs::security::SecurityCredentials;
/// # use amqprs::connection::OpenConnectionArguments;
/// // Create arguments, then update the fields.
/// let mut args = OpenConnectionArguments::new("localhost:5672","user", "bitnami")
///     .virtual_host("myhost")
///     .connection_name("myconnection")
///     .finish();
/// ```
///
/// [`Connection::open`]: struct.Connection.html#method.open
/// [`finish`]: struct.OpenConnectionArguments.html#method.finish

#[derive(Clone)]
pub struct OpenConnectionArguments {
    /// The server URI format "\<ip addr\>\:\<port\>". Default: "localhost:5672".
    uri: String,
    /// Default: "/". See [RabbitMQ vhosts](https://www.rabbitmq.com/vhosts.html).
    virtual_host: String,
    /// Default: [`None`], auto generate a connection name, otherwise use given connection name.
    connection_name: Option<String>,
    /// Default: use SASL/PLAIN authentication. See [RabbitMQ access control](https://www.rabbitmq.com/access-control.html#mechanisms).
    credentials: SecurityCredentials,
    /// Heartbeat timeout in seconds. See [RabbitMQ heartbeats](https://www.rabbitmq.com/heartbeats.html)
    /// Default: 60s. 
    heartbeat: u16,
}

impl Default for OpenConnectionArguments {
    fn default() -> Self {
        Self {
            uri: String::from("localhost:5672"),
            virtual_host: String::from("/"),
            connection_name: None,
            credentials: SecurityCredentials::new_plain("guest", "guest"),
            heartbeat: 60,
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
    pub fn new(uri: &str, username: &str, password: &str) -> Self {
        Self {
            uri: uri.to_owned(),
            virtual_host: String::from("/"),
            connection_name: None,
            credentials: SecurityCredentials::new_plain(username, password),
            heartbeat: 60,
        }
    }

    /// Set the URI of the server. Format: "\<ip addr\>\:\<port\>"
    ///
    /// # Default
    ///
    /// "localhost:5672"
    pub fn uri(&mut self, uri: &str) -> &mut Self {
        self.uri = uri.to_owned();
        self
    }

    /// Set the virtual host. See [RabbitMQ vhosts](https://www.rabbitmq.com/vhosts.html).
    ///
    /// # Default
    ///
    /// "/"
    pub fn virtual_host(&mut self, virtual_host: &str) -> &mut Self {
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
    
    /// Finish chaining and returns a new argument according to chained configurations.
    pub fn finish(&mut self) -> Self {
        self.clone()
    }
}

/////////////////////////////////////////////////////////////////////////////

impl Connection {
    /// Open and returns a new AMQP connection.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if any step goes wrong during openning an AMQP connection.
    pub async fn open(args: &OpenConnectionArguments) -> Result<Self> {
        // TODO: uri parsing
        let mut io_conn = SplitConnection::open(&args.uri).await?;

        // C:protocol-header
        Self::negotiate_protocol(&mut io_conn).await?;

        // if no given connection name, generate one
        let connection_name = match args.connection_name {
            Some(ref given_name) => given_name.clone(),
            None => generate_name(&args.uri),
        };
        // construct client properties
        let mut client_properties = AmqpPeerProperties::new();
        client_properties.insert(
            "connection_name".try_into().unwrap(),
            FieldValue::S(connection_name.clone().try_into().unwrap()),
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
        io_conn.write_frame(DEFAULT_CONN_CHANNEL, open).await?;

        // S: OpenOk
        let (_, frame) = io_conn.read_frame().await?;
        unwrap_expected_method!(
            frame,
            Frame::OpenOk,
            Error::ConnectionOpenError("open".to_string())
        )?;

        // spawn network management tasks and get internal channel' sender half.
        let (outgoing_tx, outgoing_rx) = mpsc::channel(OUTGOING_MESSAGE_BUFFER_SIZE);
        let (conn_mgmt_tx, conn_mgmt_rx) = mpsc::channel(CONNECTION_MANAGEMENT_COMMAND_BUFFER_SIZE);

        let shared = Arc::new(SharedConnectionInner {
            server_properties,
            connection_name,
            channel_max,
            frame_max,
            heartbeat,
            is_open: AtomicBool::new(true),
            outgoing_tx,
            conn_mgmt_tx,
        });

        let new_amq_conn = Self { shared };

        // spawn handlers for reader and writer of network connection
        new_amq_conn
            .spawn_handlers(io_conn, outgoing_rx, conn_mgmt_rx, heartbeat)
            .await;

        // register channel resource for connection's default channel
        new_amq_conn
            .register_channel_resource(Some(DEFAULT_CONN_CHANNEL), ChannelResource::new(None))
            .await
            .ok_or_else(|| {
                Error::ConnectionOpenError("failed to register channel resource".to_string())
            })?;
        info!("open connection: {}", new_amq_conn.connection_name());
        Ok(new_amq_conn)
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
            Error::ConnectionOpenError("start".to_string())
        )?;
        // get server supported locales
        if false
            == start
                .locales
                .as_ref()
                .split(" ")
                .any(|v| DEFAULT_LOCALE == v)
        {
            return Err(Error::ConnectionOpenError(format!(
                "locale '{}' is not supported by server",
                DEFAULT_LOCALE
            )));
        }
        // get server supported authentication mechanisms
        if false
            == start
                .mechanisms
                .as_ref()
                .split(" ")
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
            .unwrap_or(FieldValue::F(FieldTable::default()))
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
                .unwrap_or(FieldValue::S("unknown".try_into().unwrap()))
                .try_into()
                .unwrap();
            value
        };

        let server_properties = ServerProperties {
            capabilities,
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
            .write_frame(DEFAULT_CONN_CHANNEL, start_ok.into_frame())
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
            Error::ConnectionOpenError("tune".to_string())
        )?;

        // according to https://www.rabbitmq.com/heartbeats.html
        let new_heartbeat = if tune.heartbeat() == 0 || heartbeat == 0 {
            std::cmp::max(tune.heartbeat(), heartbeat)
        } else {
            std::cmp::min(tune.heartbeat(), heartbeat)
        };
        // No tunning of channel_max and frame_max
        let new_channel_max = tune.channel_max();
        let new_frame_max = tune.frame_max();
        // C: TuneOk
        let tune_ok = TuneOk::new(new_channel_max, new_frame_max, new_heartbeat);

        io_conn
            .write_frame(DEFAULT_CONN_CHANNEL, tune_ok.into_frame())
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
        self.shared.is_open.store(is_open, Ordering::Relaxed);
    }

    /// Returns `true` if connection is open.
    pub fn is_open(&self) -> bool {
        self.shared.is_open.load(Ordering::Relaxed)
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
            debug!("failed to register channel resource, cause: {}.", err);
            return None;
        }

        // expect a channel id in response
        match acker_rx.await {
            Ok(res) => {
                if let None = res {
                    debug!("failed to register channel resource, error in channel id allocation.");
                }
                res
            }
            Err(err) => {
                debug!("failed to register channel resource, cause: {}.", err);
                None
            }
        }
    }

    /// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
    pub(crate) async fn spawn_handlers(
        &self,
        connection: SplitConnection,
        outgoing_rx: mpsc::Receiver<OutgoingMessage>,
        conn_mgmt_rx: mpsc::Receiver<ConnManagementCommand>,
        heartbeat: ShortUint,
    ) {
        // Spawn two tasks for the connection
        // - one task for writer
        // - one task for reader

        let (shutdown_notifer, shutdown_listener) = broadcast::channel::<()>(1);

        let (reader, writer) = connection.into_split();

        // spawn task for read connection handler
        let rh = ReaderHandler::new(
            reader,
            self.clone(),
            self.shared.outgoing_tx.clone(),
            conn_mgmt_rx,
            self.shared.channel_max,
            shutdown_notifer,
        );
        tokio::spawn(async move {
            rh.run_until_shutdown(heartbeat).await;
        });

        // spawn task for write connection handler
        let wh = WriterHandler::new(writer, outgoing_rx, shutdown_listener);
        tokio::spawn(async move {
            wh.run_until_shutdown(heartbeat).await;
        });
    }

    /// Open and return a new AMQP channel.
    ///
    /// Automatically generate channel id if input `channel_id` is [`None`],
    /// otherwise, use the given `channel_id` if it is not occupied.
    ///
    /// If the given `channel_id` is occupied, error will be returned.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in resource allocation and communication with server.
    pub async fn open_channel(&self, channel_id: Option<AmqpChannelId>) -> Result<Channel> {
        let (dispatcher_tx, dispatcher_rx) = mpsc::channel(DISPATCHER_MESSAGE_BUFFER_SIZE);
        let (dispatcher_mgmt_tx, dispatcher_mgmt_rx) =
            mpsc::channel(DISPATCHER_MANAGEMENT_COMMAND_BUFFER_SIZE);

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
        let channel = Channel::new(
            AtomicBool::new(true),
            channel_id,
            self.shared.outgoing_tx.clone(),
            self.shared.conn_mgmt_tx.clone(),
            dispatcher_mgmt_tx,
        );

        let dispatcher = ChannelDispatcher::new(channel.clone(), dispatcher_rx, dispatcher_mgmt_rx);
        dispatcher.spawn().await;
        info!(
            "open channel: {} on connection {} ",
            channel.channel_id(),
            self.connection_name()
        );

        Ok(channel)
    }

    /// This method notify server that the connection has been blocked and does not
    /// accept new publishes.
    ///
    /// # Errors
    ///
    /// Returns error if fails to send indication to server.
    pub async fn blocked(&self, reason: &str) -> Result<()> {
        let blocked = Blocked::new(reason.clone().try_into().unwrap());

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
            self.shared
                .is_open
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            info!("close connection: {}", self.connection_name());
            self.close_internal().await?;
        }
        Ok(())
    }

    async fn close_internal(&self) -> Result<()> {
        // connection's close method , should use default channel id
        let responder_rx = self
            .register_responder(DEFAULT_CONN_CHANNEL, CloseOk::header())
            .await?;

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
}

impl Drop for Connection {
    /// When drops, try to gracefully shutdown the connection if it is still open.
    /// It is not guaranteed to succeed in a clean way.
    ///
    /// User is recommended to explicitly close connection. See [module][`self`] documentation.
    fn drop(&mut self) {
        if let Ok(true) =
            self.shared
                .is_open
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            debug!(
                "drop connection {}, spawn a task to close it.",
                self.connection_name()
            );
            let conn = self.clone();
            tokio::spawn(async move {
                info!("close connection: {} at drop", conn.connection_name());

                if let Err(err) = conn.close_internal().await {
                    // check if connection handler has exited
                    if !conn.shared.conn_mgmt_tx.is_closed() {
                        error!(
                            "{} occurred at closing connection {} after drop.",
                            err,
                            conn.connection_name()
                        );
                    }
                }
            });
        }
    }
}

/// It is uncommon to have many connections for one client
/// We only need a simple algorithm to generate large enough number of unique names.
/// To avoid using any external crate
fn generate_name(domain: &str) -> String {
    const CHAR_SET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    // at least have `usize::MAX` unique names
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    thread_local! {
        static HEAD: RefCell<usize> = RefCell::new(0);
        static TAIL: RefCell<usize> = RefCell::new(0);
    }

    let max_len = CHAR_SET.len();

    let tail = TAIL.with(|tail| {
        let current_tail = tail.borrow().to_owned();
        if current_tail + 1 == max_len {
            *tail.borrow_mut() = 0;
        } else {
            *tail.borrow_mut() += 1;
        }

        current_tail
    });

    let head = HEAD.with(|head| {
        let current_head = head.borrow().to_owned();
        if tail + 1 == max_len {
            // move HEAD index one step forward when and only when TAIL index restarts
            if current_head + 1 == max_len {
                *head.borrow_mut() = 0;
            } else {
                *head.borrow_mut() += 1;
            }
        }
        current_head
    });

    format!(
        "{}{}_{}@{}",
        char::from(CHAR_SET[head]),
        char::from(CHAR_SET[tail]),
        COUNTER.fetch_add(1, Ordering::Relaxed),
        domain
    )
}

/////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use std::{collections::HashSet, thread};

    use crate::callbacks::{DefaultChannelCallback, DefaultConnectionCallback};

    use super::{generate_name, Connection, OpenConnectionArguments};
    use tokio::time;
    use tracing::{info, subscriber::SetGlobalDefaultError, trace, Level};

    #[tokio::test]
    async fn test_channel_open_close() {
        setup_logging(Level::INFO).ok();
        {
            // test close on drop
            let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

            let connection = Connection::open(&args).await.unwrap();
            {
                // test close on drop
                let _channel = connection.open_channel(None).await.unwrap();
            }
            time::sleep(time::Duration::from_millis(10000)).await;
        }
        // wait for finished, otherwise runtime exit before all tasks are done
        time::sleep(time::Duration::from_millis(10000)).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_multi_conn_open_close() {
        setup_logging(Level::INFO).ok();

        let mut handles = vec![];

        for _ in 0..10 {
            let handle = tokio::spawn(async {
                let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_multi_channel_open_close() {
        setup_logging(Level::INFO).ok();
        {
            let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

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

    fn setup_logging(level: Level) -> Result<(), SetGlobalDefaultError> {
        // construct a subscriber that prints formatted traces to stdout
        let subscriber = tracing_subscriber::fmt().with_max_level(level).finish();

        // use that subscriber to process traces emitted after this point
        tracing::subscriber::set_global_default(subscriber)
    }

    #[test]
    fn test_name_generation() {
        let n = 100;
        let mut jh = Vec::with_capacity(n);
        let mut res = HashSet::with_capacity(n);
        for _ in 0..n {
            jh.push(thread::spawn(|| generate_name("testdomain")));
        }
        for h in jh {
            assert_eq!(true, res.insert(h.join().unwrap()));
        }
    }
}
