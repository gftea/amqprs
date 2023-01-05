use crate::{
    api::{error::Error, FieldTable},
    frame::{Bind, BindOk, Declare, DeclareOk, Delete, DeleteOk, Frame, Unbind, UnbindOk},
};

use super::{Channel, Result};

#[cfg(feature = "compilance_assert")]
use crate::api::compilance_asserts::assert_exchange_name;

/// Arguments for [`exchange_declare`]
///
/// # Support chainable methods to build arguments
/// ```
/// # use amqprs::channel::ExchangeDeclareArguments;
///
/// let x = ExchangeDeclareArguments::new("amq.direct", "direct")
///     .auto_delete(true)
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
        #[cfg(feature = "compilance_assert")]
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
    use super::{ExchangeDeclareArguments, ExchangeDeleteArguments};
    use crate::{
        api::connection::{Connection, OpenConnectionArguments},
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    };

    #[tokio::test]
    async fn test_exchange_declare() {
        let args = OpenConnectionArguments::new("localhost", Some(5672), "user", "bitnami");

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

        let args = ExchangeDeclareArguments::new("amq.topic", "topic")
            .passive(true)
            .finish();
        channel.exchange_declare(args).await.unwrap();

        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    #[should_panic = "InternalChannelError(\"channel closed\")"]
    async fn test_exchange_delete() {
        let args = OpenConnectionArguments::new("localhost", Some(5672), "user", "bitnami");

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
}
