use std::borrow::ToOwned;
use std::fmt::{Debug, Display, Formatter};
use crate::{
    api::{error::Error, FieldTable},
    frame::{Bind, BindOk, Declare, DeclareOk, Delete, DeleteOk, Frame, Unbind, UnbindOk},
};

use super::{Channel, Result};

#[cfg(feature = "compliance_assert")]
use crate::api::compliance_asserts::assert_exchange_name;

/// Exchange types. Most variants are for exchange types included with modern RabbitMQ distributions.
/// For custom types provided by 3rd party plugins, use the `Plugin(String)` variant.
#[derive(Debug, PartialEq, Eq)]
pub enum ExchangeType {
    /// Fanout exchange
    Fanout,
    /// Topic exchange
    Topic,
    /// Direct exchange
    Direct,
    /// Headers exchange
    Headers,
    /// Consistent hashing (consistent hash) exchange
    ConsistentHashing,
    /// Modulus hash, ships with the 'rabbitmq-sharding' plugin
    ModulusHash,
    /// Random exchange
    Random,
    /// JMS topic exchange
    JmsTopic,
    /// Recent history exchange
    RecentHistory,
    /// All other x-* exchange types, for example, those provided by plugins
    Plugin(String)
}

const EXCHANGE_TYPE_FANOUT: &str = "fanout";
const EXCHANGE_TYPE_TOPIC:  &str = "topic";
const EXCHANGE_TYPE_DIRECT:  &str = "direct";
const EXCHANGE_TYPE_HEADERS:  &str = "headers";
const EXCHANGE_TYPE_CONSISTENT_HASHING:  &str = "x-consistent-hash";
const EXCHANGE_TYPE_MODULUS_HASH:  &str = "x-modulus-hash";
const EXCHANGE_TYPE_RANDOM:  &str = "x-random";
const EXCHANGE_TYPE_JMS_TOPIC:  &str = "x-jms-topic";
const EXCHANGE_TYPE_RECENT_HISTORY:  &str = "x-recent-history";

impl From<&str> for ExchangeType {
    fn from(value: &str) -> Self {
        match value {
            EXCHANGE_TYPE_FANOUT => ExchangeType::Fanout,
            EXCHANGE_TYPE_TOPIC => ExchangeType::Topic,
            EXCHANGE_TYPE_DIRECT => ExchangeType::Direct,
            EXCHANGE_TYPE_HEADERS => ExchangeType::Headers,
            EXCHANGE_TYPE_CONSISTENT_HASHING => ExchangeType::ConsistentHashing,
            EXCHANGE_TYPE_MODULUS_HASH => ExchangeType::ModulusHash,
            EXCHANGE_TYPE_RANDOM => ExchangeType::Random,
            EXCHANGE_TYPE_JMS_TOPIC => ExchangeType::JmsTopic,
            EXCHANGE_TYPE_RECENT_HISTORY => ExchangeType::RecentHistory,
            other => ExchangeType::Plugin(other.to_owned())
        }
    }
}

impl From<String> for ExchangeType {
    fn from(value: String) -> Self {
        ExchangeType::from(value.as_str())
    }
}

impl From<ExchangeType> for String {
    fn from(value: ExchangeType) -> String {
        match value {
            ExchangeType::Fanout => EXCHANGE_TYPE_FANOUT.to_owned(),
            ExchangeType::Topic => EXCHANGE_TYPE_TOPIC.to_owned(),
            ExchangeType::Direct => EXCHANGE_TYPE_DIRECT.to_owned(),
            ExchangeType::Headers => EXCHANGE_TYPE_HEADERS.to_owned(),
            ExchangeType::ConsistentHashing => EXCHANGE_TYPE_CONSISTENT_HASHING.to_owned(),
            ExchangeType::ModulusHash => EXCHANGE_TYPE_MODULUS_HASH.to_owned(),
            ExchangeType::Random => EXCHANGE_TYPE_RANDOM.to_owned(),
            ExchangeType::JmsTopic => EXCHANGE_TYPE_JMS_TOPIC.to_owned(),
            ExchangeType::RecentHistory => EXCHANGE_TYPE_RECENT_HISTORY.to_owned(),
            ExchangeType::Plugin(exchange_type) => exchange_type
        }
    }
}

impl Display for ExchangeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangeType::Fanout => Display::fmt(&EXCHANGE_TYPE_FANOUT, f),
            ExchangeType::Topic => Display::fmt(&EXCHANGE_TYPE_TOPIC, f),
            ExchangeType::Direct => Display::fmt(&EXCHANGE_TYPE_DIRECT, f),
            ExchangeType::Headers => Display::fmt(&EXCHANGE_TYPE_HEADERS, f),
            ExchangeType::ConsistentHashing => Display::fmt(&EXCHANGE_TYPE_CONSISTENT_HASHING, f),
            ExchangeType::ModulusHash => Display::fmt(&EXCHANGE_TYPE_MODULUS_HASH, f),
            ExchangeType::Random => Display::fmt(&EXCHANGE_TYPE_RANDOM, f),
            ExchangeType::JmsTopic => Display::fmt(&EXCHANGE_TYPE_JMS_TOPIC, f),
            ExchangeType::RecentHistory => Display::fmt(&EXCHANGE_TYPE_RECENT_HISTORY,f),
            ExchangeType::Plugin(exchange_type) => Display::fmt(&exchange_type, f)
        }
    }
}

/// Arguments for [`exchange_declare`]
///
/// # Support chainable methods to build arguments
/// ```
/// # use amqprs::channel::{ExchangeDeclareArguments, ExchangeType};
///
/// let x = ExchangeDeclareArguments::of_type("amq.direct", ExchangeType::Direct)
///     .durable(true)
///     .finish();
/// ```
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.declare).
///
/// [`exchange_declare`]: struct.Channel.html#method.exchange_declare
#[derive(Debug, Clone)]
pub struct ExchangeDeclareArguments {
    /// Exchange name. Default: "".
    pub exchange: String,
    /// Default: "direct".
    pub exchange_type: String,
    /// Default: `false`.
    pub passive: bool,
    /// Default: `false`.
    pub durable: bool,
    /// Default: `false`.
    pub auto_delete: bool,
    /// Default: `false`.
    pub internal: bool,
    /// Default: `false`.
    pub no_wait: bool,
    /// Default: empty table.
    pub arguments: FieldTable,
}

impl Default for ExchangeDeclareArguments {
    fn default() -> Self {
        Self {
            exchange: Default::default(),
            exchange_type: "direct".to_owned(),
            passive: Default::default(),
            durable: Default::default(),
            auto_delete: Default::default(),
            internal: Default::default(),
            no_wait: Default::default(),
            arguments: Default::default(),
        }
    }
}

impl ExchangeDeclareArguments {
    /// Creates new arguments with defaults
    pub fn new(exchange: &str, exchange_type: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_exchange_name(exchange);

        Self {
            exchange: exchange.to_string(),
            exchange_type: exchange_type.to_string(),
            passive: false,
            durable: false,
            auto_delete: false,
            internal: false,
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }

    // Creates new arguments with defaults
    pub fn of_type(exchange: &str, exchange_type: ExchangeType) -> Self {
        let s: String = Into::<String>::into(exchange_type);
        Self::new(exchange, &s)
    }

    impl_chainable_setter! {
        /// Chainable setter method.
        exchange, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        exchange_type, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        passive, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        durable, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        auto_delete, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        internal, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_wait, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        arguments, FieldTable
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_exchange_name(&self.exchange);

        self.clone()
    }
}

/// Arguments for [`exchange_delete`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.delete).
///
/// [`exchange_delete`]: struct.Channel.html#method.exchange_delete
#[derive(Debug, Clone, Default)]
pub struct ExchangeDeleteArguments {
    /// Exchange name. Default: "".
    pub exchange: String,
    /// Default: `false`.
    pub if_unused: bool,
    /// Default: `false`.
    pub no_wait: bool,
}

impl ExchangeDeleteArguments {
    /// Create new arguments with defaults
    pub fn new(exchange: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_exchange_name(exchange);

        Self {
            exchange: exchange.to_owned(),
            if_unused: false,
            no_wait: false,
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        exchange, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        if_unused, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_wait, bool
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_exchange_name(&self.exchange);

        self.clone()
    }
}

/// Arguments for [`exchange_bind`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.bind).
///
/// [`exchange_bind`]: struct.Channel.html#method.exchange_bind
#[derive(Debug, Clone, Default)]
pub struct ExchangeBindArguments {
    /// Destination exchange name. Default: "".
    pub destination: String,
    /// Source exchange name. Default: "".
    pub source: String,
    /// Default: "".
    pub routing_key: String,
    /// Default: `false`.
    pub no_wait: bool,
    /// Default: empty table.
    pub arguments: FieldTable,
}

impl ExchangeBindArguments {
    /// Create arguments with defaults
    pub fn new(destination: &str, source: &str, routing_key: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        {
            assert_exchange_name(destination);
            assert_exchange_name(source);
        }
        Self {
            destination: destination.to_owned(),
            source: source.to_owned(),
            routing_key: routing_key.to_owned(),
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        destination, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        source, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        routing_key, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_wait, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        arguments, FieldTable
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        {
            assert_exchange_name(&self.destination);
            assert_exchange_name(&self.source);
        }
        self.clone()
    }
}

/// Arguments for [`exchange_unbind`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.unbind).
///
/// [`exchange_unbind`]: struct.Channel.html#method.exchange_unbind
#[derive(Debug, Clone, Default)]
pub struct ExchangeUnbindArguments {
    /// Destination exchange name. Default: "".
    pub destination: String,
    /// Source exchange name. Default: "".
    pub source: String,
    /// Default: "".
    pub routing_key: String,
    /// Default: `false`.
    pub no_wait: bool,
    /// Default: empty table
    pub arguments: FieldTable,
}

impl ExchangeUnbindArguments {
    /// Create arguments with defaults
    pub fn new(destination: &str, source: &str, routing_key: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        {
            assert_exchange_name(destination);
            assert_exchange_name(source);
        }
        Self {
            destination: destination.to_string(),
            source: source.to_string(),
            routing_key: routing_key.to_string(),
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        destination, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        source, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        routing_key, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_wait, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        arguments, FieldTable
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        {
            assert_exchange_name(&self.destination);
            assert_exchange_name(&self.source);
        }
        self.clone()
    }
}
/////////////////////////////////////////////////////////////////////////////
/// APIs for AMQP exchange class
impl Channel {
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.declaure)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn exchange_declare(&self, args: ExchangeDeclareArguments) -> Result<()> {
        let mut declare = Declare::new(
            0,
            args.exchange.try_into().unwrap(),
            args.exchange_type.try_into().unwrap(),
            args.arguments,
        );

        declare.set_passive(args.passive);
        declare.set_durable(args.durable);
        declare.set_auto_delete(args.auto_delete);
        declare.set_internal(args.internal);
        declare.set_no_wait(args.no_wait);

        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, declare.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(DeclareOk::header()).await?;

            let _method = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, declare.into_frame()),
                responder_rx,
                Frame::DeclareOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.delete)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn exchange_delete(&self, args: ExchangeDeleteArguments) -> Result<()> {
        let mut delete = Delete::new(0, args.exchange.try_into().unwrap());
        delete.set_if_unused(args.if_unused);
        delete.set_no_wait(args.no_wait);
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, delete.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(DeleteOk::header()).await?;

            let _method = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, delete.into_frame()),
                responder_rx,
                Frame::DeleteOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.bind)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn exchange_bind(&self, args: ExchangeBindArguments) -> Result<()> {
        let bind = Bind::new(
            0,
            args.destination.try_into().unwrap(),
            args.source.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.no_wait,
            args.arguments,
        );
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, bind.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(BindOk::header()).await?;

            synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, bind.into_frame()),
                responder_rx,
                Frame::BindOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#exchange.unbind)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn exchange_unbind(&self, args: ExchangeUnbindArguments) -> Result<()> {
        let unbind = Unbind::new(
            0,
            args.destination.try_into().unwrap(),
            args.source.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.no_wait,
            args.arguments,
        );
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, unbind.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(UnbindOk::header()).await?;

            synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, unbind.into_frame()),
                responder_rx,
                Frame::UnbindOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExchangeBindArguments, ExchangeDeclareArguments, ExchangeDeleteArguments,
        ExchangeUnbindArguments, ExchangeType,
    };
    use crate::{
        api::connection::{Connection, OpenConnectionArguments},
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        test_utils,
    };

    #[tokio::test]
    async fn test_exchange_type() {
        assert_eq!(ExchangeType::Fanout.to_string(), "fanout");
        assert_eq!(ExchangeType::Topic.to_string(), "topic");
        assert_eq!(ExchangeType::Direct.to_string(), "direct");
        assert_eq!(ExchangeType::Headers.to_string(), "headers");
        assert_eq!(ExchangeType::ConsistentHashing.to_string(), "x-consistent-hash");
        assert_eq!(ExchangeType::Random.to_string(), "x-random");
        assert_eq!(ExchangeType::JmsTopic.to_string(), "x-jms-topic");
        assert_eq!(ExchangeType::RecentHistory.to_string(), "x-recent-history");
        assert_eq!(ExchangeType::ModulusHash.to_string(), "x-modulus-hash");
        assert_eq!(ExchangeType::Plugin(String::from("x-custom-exchange-2")).to_string(), "x-custom-exchange-2");

        assert_eq!(ExchangeType::from("fanout"), ExchangeType::Fanout);
        assert_eq!(ExchangeType::from("topic"), ExchangeType::Topic);
        assert_eq!(ExchangeType::from("direct"), ExchangeType::Direct);
        assert_eq!(ExchangeType::from("headers"), ExchangeType::Headers);
        assert_eq!(ExchangeType::from("x-consistent-hash"), ExchangeType::ConsistentHashing);
        assert_eq!(ExchangeType::from("x-random"), ExchangeType::Random);
        assert_eq!(ExchangeType::from("x-jms-topic"), ExchangeType::JmsTopic);
        assert_eq!(ExchangeType::from("x-modulus-hash"), ExchangeType::ModulusHash);
        assert_eq!(ExchangeType::from("x-custom-exchange-2"), ExchangeType::Plugin(String::from("x-custom-exchange-2")));

        assert_eq!(String::from(ExchangeType::Fanout), "fanout");
        assert_eq!(String::from(ExchangeType::Topic), "topic");
        assert_eq!(String::from(ExchangeType::Direct), "direct");
        assert_eq!(String::from(ExchangeType::Headers), "headers");
        assert_eq!(String::from(ExchangeType::ModulusHash), "x-modulus-hash");
        assert_eq!(String::from(ExchangeType::ConsistentHashing), "x-consistent-hash");
        assert_eq!(String::from(ExchangeType::RecentHistory), "x-recent-history");
        assert_eq!(String::from(ExchangeType::Random), "x-random");
        assert_eq!(String::from(ExchangeType::JmsTopic), "x-jms-topic");
        assert_eq!(String::from(ExchangeType::Plugin(String::from("x-custom-exchange-3"))), "x-custom-exchange-3");

    }

    #[tokio::test]
    async fn test_exchange_declare() {
        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

        let connection = Connection::open(&args).await.unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();

        let args = ExchangeDeclareArguments::of_type("amq.topic", ExchangeType::Topic)
            .passive(true)
            .finish();
        channel.exchange_declare(args).await.unwrap();

        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    #[should_panic = "InternalChannelError(\"channel closed\")"]
    async fn test_exchange_delete() {
        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

        let connection = Connection::open(&args).await.unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();
        let args = ExchangeDeleteArguments::new("amq.direct");
        channel.exchange_delete(args).await.unwrap();
    }

    #[tokio::test]
    async fn test_exchange_bind_unbind() {
        test_utils::setup_logging();
        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

        let connection = Connection::open(&args).await.unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();

        let source = "primary";
        channel
            .exchange_declare(
                ExchangeDeclareArguments::default()
                    .exchange(source.to_owned())
                    .exchange_type("topic".to_owned())
                    .finish(),
            )
            .await
            .unwrap();

        let dest = "secondary";
        channel
            .exchange_declare(
                ExchangeDeclareArguments::default()
                    .exchange(dest.to_owned())
                    .exchange_type("topic".to_owned())
                    .finish(),
            )
            .await
            .unwrap();

        let routing_key = "p.s.proxy";
        channel
            .exchange_bind(ExchangeBindArguments::new(dest, source, routing_key))
            .await
            .unwrap();

        channel
            .exchange_unbind(ExchangeUnbindArguments::new(dest, source, routing_key))
            .await
            .unwrap();
    }
}
