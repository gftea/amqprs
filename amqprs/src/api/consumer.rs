use std::str::from_utf8;

use amqp_serde::types::AmqpChannelId;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::{
    frame::{Ack, BasicProperties, Deliver},
    net::OutgoingMessage,
};

use super::channel::{Acker, BasicAckArguments};

#[async_trait]
pub trait Consumer {
    // fn is_auto_ack(&self) -> bool;
    async fn consume(
        &mut self,
        acker: Option<&Acker>,
        deliver: Deliver,
        basic_propertities: BasicProperties,
        content: Vec<u8>,
    );
}

pub struct DefaultConsumer;

#[async_trait]
impl Consumer for DefaultConsumer {
    // fn is_auto_ack(&self) -> bool {
    //     false
    // }

    async fn consume(
        &mut self,
        acker: Option<&Acker>,
        deliver: Deliver,
        basic_propertities: BasicProperties,
        content: Vec<u8>,
    ) {
        println!(">>>>> Consumer Start <<<<<<");
        println!("{:?}", deliver,);
        println!("{:?}", basic_propertities,);
        println!("{}", from_utf8(&content).unwrap());
        println!(">>>>> Consumer End <<<<<<");

        // none if auto ack
        if let Some(acker) = acker {
            acker.basic_ack(BasicAckArguments::new(deliver.delivery_tag())).await.unwrap();
        }
    }
}
