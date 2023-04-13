use amqp_serde::types::AmqpMessageCount;

use super::Channel;
use crate::{
    api::{error::Error, FieldTable, Result},
    frame::{
        BindQueue, BindQueueOk, DeclareQueue, DeclareQueueOk, DeleteQueue, DeleteQueueOk, Frame,
        PurgeQueue, PurgeQueueOk, UnbindQueue, UnbindQueueOk,
    },
};

#[cfg(feature = "compliance_assert")]
use crate::api::compliance_asserts::{assert_exchange_name, assert_queue_name};

////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`queue_declare`]
///
/// # Support chainable methods to build arguments
/// ```
/// # use amqprs::channel::QueueDeclareArguments;
///
/// let x = QueueDeclareArguments::default()
///     .auto_delete(true)
///     .durable(true)
///     .finish();
/// ```
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.declare).
///
/// [`queue_declare`]: struct.Channel.html#method.queue_declare
#[derive(Debug, Clone, Default)]
pub struct QueueDeclareArguments {
    /// Queue name. Default: "".
    queue: String,
    /// Default: `false`.
    passive: bool,
    /// Default: `false`.
    durable: bool,
    /// Default: `false`.
    exclusive: bool,
    /// Default: `false`.
    auto_delete: bool,
    /// Default: `false`.
    no_wait: bool,
    /// Default: empty table.
    arguments: FieldTable,
}

impl QueueDeclareArguments {
    /// Default arguments: declares a transient, client-named, non-exclusive and non-autodelete queue
    pub fn new(queue: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(queue);

        Self {
            queue: queue.to_owned(),
            passive: false,
            durable: false,
            exclusive: false,
            auto_delete: false,
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }

    /// Arguments of a durable, non-exclusive, non-autodelete
    // queue. Usually a good fit for queues with well-known names.
    pub fn durable_client_named(queue: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(queue);

        Self {
            queue: queue.to_owned(),
            passive: false,
            durable: true,
            exclusive: false,
            auto_delete: false,
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }

    /// Arguments of an exclusive, transient, server-named
    // queue. Usually a good fit for queues that store client-specific transient state.
    pub fn exclusive_server_named() -> Self {
        Self {
            queue: "".to_owned(),
            passive: false,
            durable: false,
            exclusive: true,
            auto_delete: false,
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }

    /// Arguments of an autodelete, transient, client-named
    // queue. Usually a good fit for queues that store client-specific transient state
    // when server-named queues are not an option.
    pub fn transient_autodelete(queue: &str) -> Self {
        Self {
            queue: queue.to_owned(),
            passive: false,
            durable: false,
            exclusive: false,
            auto_delete: true,
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }

    //-------------------------------------------------------------------------
    impl_chainable_setter! {
        /// Chainable setter method.
        queue, String
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
        exclusive, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        auto_delete, bool
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
        assert_queue_name(&self.queue);

        self.clone()
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`queue_bind`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.bind).
///
/// [`queue_bind`]: struct.Channel.html#method.queue_bind
#[derive(Debug, Clone, Default)]
pub struct QueueBindArguments {
    /// Queue name. Default: "".
    pub queue: String,
    /// Exchange name. Default: "".
    pub exchange: String,
    /// Default: "".
    pub routing_key: String,
    /// Default: `false`.
    pub no_wait: bool,
    /// Default: empty table.
    pub arguments: FieldTable,
}

impl QueueBindArguments {
    /// Create new arguments with defaults.
    pub fn new(queue: &str, exchange: &str, routing_key: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        {
            assert_queue_name(queue);
            assert_exchange_name(exchange);
        }

        Self {
            queue: queue.to_owned(),
            exchange: exchange.to_owned(),
            routing_key: routing_key.to_owned(),
            no_wait: false,
            arguments: FieldTable::new(),
        }
    }
    //-------------------------------------------------------------------------
    impl_chainable_setter! {
        /// Chainable setter method.
        queue, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        exchange, String
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
            assert_queue_name(&self.queue);
            assert_exchange_name(&self.exchange);
        }

        self.clone()
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`queue_purge`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.purge).
///
/// [`queue_purge`]: struct.Channel.html#method.queue_purge
#[derive(Debug, Clone, Default)]
pub struct QueuePurgeArguments {
    /// Queue name. Default: "".
    pub queue: String,
    /// Default: `false`.
    pub no_wait: bool,
}

impl QueuePurgeArguments {
    /// Create new arguments with defaults.
    pub fn new(queue: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(queue);

        Self {
            queue: queue.to_owned(),
            no_wait: false,
        }
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`queue_delete`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.delete).
///
/// [`queue_delete`]: struct.Channel.html#method.queue_delete
#[derive(Debug, Clone, Default)]
pub struct QueueDeleteArguments {
    /// Queue name. Default: "".
    pub queue: String,
    /// Default: `false`.
    pub if_unused: bool,
    /// Default: `false`.
    pub if_empty: bool,
    /// Default: `false`.
    pub no_wait: bool,
}

impl QueueDeleteArguments {
    /// Create new arguments with defaults.
    pub fn new(queue: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(queue);

        Self {
            queue: queue.to_owned(),
            if_unused: false,
            if_empty: false,
            no_wait: false,
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        queue, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        if_unused, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        if_empty, bool
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        no_wait, bool
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        assert_queue_name(&self.queue);

        self.clone()
    }
}
////////////////////////////////////////////////////////////////////////////////
/// Arguments for [`queue_unbind`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.unbind).
///
/// [`queue_unbind`]: struct.Channel.html#method.queue_unbind
#[derive(Debug, Clone, Default)]
pub struct QueueUnbindArguments {
    /// Queue name. Default: "".
    pub queue: String,
    /// Exchange name. Default: "".
    pub exchange: String,
    /// Default: "".
    pub routing_key: String,
    /// Default: empty table.
    pub arguments: FieldTable,
}

impl QueueUnbindArguments {
    /// Create new arguments with defaults.
    pub fn new(queue: &str, exchange: &str, routing_key: &str) -> Self {
        #[cfg(feature = "compliance_assert")]
        {
            assert_queue_name(queue);
            assert_exchange_name(exchange);
        }

        Self {
            queue: queue.to_owned(),
            exchange: exchange.to_owned(),
            routing_key: routing_key.to_owned(),
            arguments: FieldTable::new(),
        }
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        queue, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        exchange, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        routing_key, String
    }
    impl_chainable_setter! {
        /// Chainable setter method.
        arguments, FieldTable
    }
    /// Finish chained configuration and return new arguments.
    pub fn finish(&mut self) -> Self {
        #[cfg(feature = "compliance_assert")]
        {
            assert_queue_name(&self.queue);
            assert_exchange_name(&self.exchange);
        }

        self.clone()
    }
}

/////////////////////////////////////////////////////////////////////////////
/// APIs for AMQP queue class.
impl Channel {
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.declare)
    ///
    /// If succeed, returns [`Ok`] with a optional tuple.
    ///
    /// Returns a tuple `(queue_name, message_count, consumer_count)`
    /// if `no_wait` argument is `false`, otherwise returns [`None`].
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn queue_declare(
        &self,
        args: QueueDeclareArguments,
    ) -> Result<Option<(String, AmqpMessageCount, u32)>> {
        let mut declare = DeclareQueue::new(0, args.queue.try_into().unwrap(), args.arguments);
        declare.set_passive(args.passive);
        declare.set_durable(args.durable);
        declare.set_exclusive(args.exclusive);
        declare.set_auto_delete(args.auto_delete);
        declare.set_no_wait(args.no_wait);
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.channel_id(), declare.into_frame()))
                .await?;
            Ok(None)
        } else {
            let responder_rx = self.register_responder(DeclareQueueOk::header()).await?;
            let delcare_ok = synchronous_request!(
                self.shared.outgoing_tx,
                (self.channel_id(), declare.into_frame()),
                responder_rx,
                Frame::DeclareQueueOk,
                Error::ChannelUseError
            )?;
            Ok(Some((
                delcare_ok.queue.into(),
                delcare_ok.message_count,
                delcare_ok.consumer_count,
            )))
        }
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.bind)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn queue_bind(&self, args: QueueBindArguments) -> Result<()> {
        let bind = BindQueue::new(
            0,
            args.queue.try_into().unwrap(),
            args.exchange.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.no_wait,
            args.arguments,
        );

        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.channel_id(), bind.into_frame()))
                .await?;
        } else {
            let responder_rx = self.register_responder(BindQueueOk::header()).await?;

            synchronous_request!(
                self.shared.outgoing_tx,
                (self.channel_id(), bind.into_frame()),
                responder_rx,
                Frame::BindQueueOk,
                Error::ChannelUseError
            )?;
        }
        Ok(())
    }

    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.purge)
    ///
    /// If succeed, returns [`Ok`] with a optional `message count`.
    ///
    /// Returns `message count` if `no_wait` argument is `false`, otherwise returns [`None`].
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn queue_purge(&self, args: QueuePurgeArguments) -> Result<Option<AmqpMessageCount>> {
        let purge = PurgeQueue::new(0, args.queue.try_into().unwrap(), args.no_wait);

        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.channel_id(), purge.into_frame()))
                .await?;
            Ok(None)
        } else {
            let responder_rx = self.register_responder(PurgeQueueOk::header()).await?;

            let purge_ok = synchronous_request!(
                self.shared.outgoing_tx,
                (self.channel_id(), purge.into_frame()),
                responder_rx,
                Frame::PurgeQueueOk,
                Error::ChannelUseError
            )?;
            Ok(Some(purge_ok.message_count))
        }
    }
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.delete)
    ///
    /// If succeed, returns [`Ok`] with a optional `message count`.
    ///
    /// Returns `message count` if `no_wait` argument is `false`, otherwise returns [`None`].
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn queue_delete(
        &self,
        args: QueueDeleteArguments,
    ) -> Result<Option<AmqpMessageCount>> {
        let mut delete = DeleteQueue::new(0, args.queue.try_into().unwrap());
        delete.set_if_unused(args.if_unused);
        delete.set_if_empty(args.if_empty);
        delete.set_no_wait(args.no_wait);
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.channel_id(), delete.into_frame()))
                .await?;
            Ok(None)
        } else {
            let responder_rx = self.register_responder(DeleteQueueOk::header()).await?;

            let delete_ok = synchronous_request!(
                self.shared.outgoing_tx,
                (self.channel_id(), delete.into_frame()),
                responder_rx,
                Frame::DeleteQueueOk,
                Error::ChannelUseError
            )?;
            Ok(Some(delete_ok.message_count))
        }
    }
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#queue.unbind)
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn queue_unbind(&self, args: QueueUnbindArguments) -> Result<()> {
        let unbind = UnbindQueue::new(
            0,
            args.queue.try_into().unwrap(),
            args.exchange.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.arguments,
        );

        let responder_rx = self.register_responder(UnbindQueueOk::header()).await?;

        synchronous_request!(
            self.shared.outgoing_tx,
            (self.channel_id(), unbind.into_frame()),
            responder_rx,
            Frame::UnbindQueueOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        connection::{Connection, OpenConnectionArguments},
    };

    use super::{
        QueueBindArguments, QueueDeclareArguments, QueueDeleteArguments, QueuePurgeArguments,
        QueueUnbindArguments,
    };

    #[tokio::test]
    async fn test_queue_apis() {
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

        let (queue_name, ..) = channel
            .queue_declare(QueueDeclareArguments::default())
            .await
            .unwrap()
            .unwrap();
        channel
            .queue_bind(QueueBindArguments::new(
                &queue_name,
                "amq.topic",
                "eiffel.#",
            ))
            .await
            .unwrap();

        // purge
        channel
            .queue_purge(QueuePurgeArguments::new(&queue_name))
            .await
            .unwrap();

        // unbind
        channel
            .queue_unbind(QueueUnbindArguments::new(
                &queue_name,
                "amq.topic",
                "eiffel.#",
            ))
            .await
            .unwrap();

        // delete
        channel
            .queue_delete(QueueDeleteArguments::new(&queue_name))
            .await
            .unwrap();

        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
