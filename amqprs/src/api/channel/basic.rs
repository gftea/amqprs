use std::collections::BTreeMap;

use crate::{
    api::error::Error,
    frame::{Ack, BasicPropertities, Consume, Frame, Qos},
};

use super::{Channel, Result, ServerSpecificArguments};

#[derive(Debug, Clone)]
pub struct BasicQosArguments {
    pub prefetch_size: u32,
    pub prefetch_count: u16,
    pub global: bool,
}

impl BasicQosArguments {
    pub fn new() -> Self {
        Self {
            prefetch_size: 0,
            prefetch_count: 0,
            global: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicConsumeArguments {
    queue: String,
    consumer_tag: String,
    no_local: bool,
    no_ack: bool,
    exclusive: bool,
    no_wait: bool,
    arguments: ServerSpecificArguments,
}

impl BasicConsumeArguments {
    pub fn new(queue: &str, consumer_tag: &str) -> Self {
        Self {
            queue: queue.to_string(),
            consumer_tag: consumer_tag.to_string(),
            no_local: false,
            no_ack: false,
            exclusive: false,
            no_wait: false,
            arguments: ServerSpecificArguments::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicAckArguments {
    pub delivery_tag: u64,
    pub multiple: bool,
}

impl BasicAckArguments {
    pub fn new() -> Self {
        Self {
            delivery_tag: 0,
            multiple: false,
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    pub async fn basic_qos(&mut self, args: BasicQosArguments) -> Result<()> {
        let qos = Qos {
            prefetch_size: args.prefetch_size,
            prefetch_count: args.prefetch_count,
            global: args.global,
        };
        let _method = synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, qos.into_frame()),
            self.incoming_rx,
            Frame::QosOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    /// return consumer tag
    pub async fn basic_consume<F>(
        &mut self,
        args: BasicConsumeArguments,
        handler: F,
    ) -> Result<String>
    where
        // TODO: this is blocking callback, spawn blocking task in connection manager
        // to provide async callback, and spawn async task  in connection manager
        F: Fn() -> () + Send + Sync + 'static,
    {
        let BasicConsumeArguments {
            queue,
            consumer_tag,
            no_local,
            no_ack,
            exclusive,
            no_wait,
            arguments,
        } = args;

        let mut consume = Consume {
            ticket: 0,
            queue: queue.try_into().unwrap(),
            consumer_tag: consumer_tag.clone().try_into().unwrap(),
            bits: 0,
            arguments: arguments.into_field_table(),
        };
        consume.set_no_local(no_local);
        consume.set_no_ack(no_ack);
        consume.set_exclusive(exclusive);
        consume.set_nowait(no_wait);

        // TODO: register consumer
        

        if args.no_wait {
            self.outgoing_tx
                .send((self.channel_id, consume.into_frame()))
                .await?;
            Ok(consumer_tag)
        } else {
            let method = synchronous_request!(
                self.outgoing_tx,
                (self.channel_id, consume.into_frame()),
                self.incoming_rx,
                Frame::ConsumeOk,
                Error::ChannelUseError
            )?;
            Ok(method.consumer_tag.into())
        }
    }

    pub async fn basic_ack(&mut self, args: BasicAckArguments) -> Result<()> {
        let ack = Ack {
            delivery_tag: args.delivery_tag,
            mutiple: args.multiple,
        };
        self.outgoing_tx.send((self.channel_id, ack.into_frame())).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::{connection::Connection, channel::{QueueDeclareArguments, QueueBindArguments}};

    use super::BasicConsumeArguments;

    #[tokio::test]
    async fn test_basic_consume() {
        let mut client = Connection::open("localhost:5672").await.unwrap();

        let mut channel = client.open_channel().await.unwrap();
        channel.queue_declare(QueueDeclareArguments::new("queue")).await.unwrap();
        channel.queue_bind(QueueBindArguments::new("queue", "amq.direct", "")).await.unwrap();
        channel
            .basic_consume(BasicConsumeArguments::new("", ""), || {
                println!("consume a message!");
            })
            .await
            .unwrap();
    }
}
