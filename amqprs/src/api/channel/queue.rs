use amqp_serde::types::AmqpMessageCount;

use super::{Channel, AmqArgumentTable};
use crate::{
    api::{error::Error, Result},
    frame::{
        BindQueue, BindQueueOk, DeclareQueue, DeclareQueueOk, DeleteQueue, DeleteQueueOk, Frame,
        PurgeQueue, PurgeQueueOk, UnbindQueue, UnbindQueueOk,
    },
};

#[derive(Debug, Clone)]
pub struct QueueDeclareArguments {
    pub queue: String,
    pub passive: bool,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub no_wait: bool,
    pub arguments: AmqArgumentTable,
}

impl QueueDeclareArguments {
    pub fn new(queue: &str) -> Self {
        Self {
            queue: queue.to_string(),
            passive: false,
            durable: false,
            exclusive: false,
            auto_delete: false,
            no_wait: false,
            arguments: AmqArgumentTable::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueBindArguments {
    pub queue: String,
    pub exchange: String,
    pub routing_key: String,
    pub no_wait: bool,
    pub arguments: AmqArgumentTable,
}

impl QueueBindArguments {
    pub fn new(queue: &str, exchange: &str, routing_key: &str) -> Self {
        Self {
            queue: queue.to_string(),
            exchange: exchange.to_string(),
            routing_key: routing_key.to_string(),
            no_wait: false,
            arguments: AmqArgumentTable::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueuePurgeArguments {
    pub queue: String,
    pub no_wait: bool,
}

impl QueuePurgeArguments {
    pub fn new(queue: &str) -> Self {
        Self {
            queue: queue.to_string(),
            no_wait: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueDeleteArguments {
    pub queue: String,
    pub if_unused: bool,
    pub if_empty: bool,
    pub no_wait: bool,
}

impl QueueDeleteArguments {
    pub fn new(queue: &str) -> Self {
        Self {
            queue: queue.to_string(),
            if_unused: false,
            if_empty: false,
            no_wait: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueUnbindArguments {
    pub queue: String,
    pub exchange: String,
    pub routing_key: String,
    pub arguments: AmqArgumentTable,
}

impl QueueUnbindArguments {
    pub fn new(queue: &str, exchange: &str, routing_key: &str) -> Self {
        Self {
            queue: queue.to_string(),
            exchange: exchange.to_string(),
            routing_key: routing_key.to_string(),
            arguments: AmqArgumentTable::new(),
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    /// When result is Ok
    ///     - no_wait = false, which means require synchronous response, return `(queue_name, message_count, consumer_count)` wrapped in `Some`
    ///     - no_wait = true, which means no response required, return `None`
    ///
    pub async fn queue_declare(
        &self,
        args: QueueDeclareArguments,
    ) -> Result<Option<(String, AmqpMessageCount, u32)>> {
        let mut declare = DeclareQueue::new(
            0,
            args.queue.try_into().unwrap(),
            0,
            args.arguments.into_field_table(),
        );
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

    ///
    pub async fn queue_bind(&self, args: QueueBindArguments) -> Result<()> {
        let bind = BindQueue::new(
            0,
            args.queue.try_into().unwrap(),
            args.exchange.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.no_wait,
            args.arguments.into_field_table(),
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

    /// When result is Ok
    ///     - no_wait = false, which means require synchronous response, return message count wrapped in `Some`
    ///     - no_wait = true, which means no response required, return `None`
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
            Ok(Some(purge_ok.message_count()))
        }
    }

    /// When result is Ok
    ///     - no_wait = false, which means require synchronous response, return message count wrapped in `Some`
    ///     - no_wait = true, which means no response required, return `None`
    pub async fn queue_delete(
        &self,
        args: QueueDeleteArguments,
    ) -> Result<Option<AmqpMessageCount>> {
        let mut delete = DeleteQueue::new(0, args.queue.try_into().unwrap(), 0);
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
            Ok(Some(delete_ok.message_count()))
        }
    }

    pub async fn queue_unbind(&self, args: QueueUnbindArguments) -> Result<()> {
        let unbind = UnbindQueue::new(
            0,
            args.queue.try_into().unwrap(),
            args.exchange.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.arguments.into_field_table(),
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
