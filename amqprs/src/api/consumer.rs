use std::str::from_utf8;

use async_trait::async_trait;
use tracing::info;

use crate::frame::{BasicProperties, Deliver};

use super::channel::{BasicAckArguments, Channel};

#[async_trait]
pub trait Consumer {
    // fn is_auto_ack(&self) -> bool;
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    );
}

pub struct DefaultConsumer {
    no_ack: bool,
}

impl DefaultConsumer {
    pub fn new(no_ack: bool) -> Self {
        Self { no_ack }
    }
}

#[async_trait]
impl Consumer for DefaultConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        info!(">>>>> Consumer '{}' Start <<<<<", deliver.consumer_tag());
        info!("{:?}", deliver,);
        info!("{:?}", basic_properties,);
        info!("{}", from_utf8(&content).unwrap());
        info!(">>>>> Consumer '{}' End <<<<<", deliver.consumer_tag());

        // ack explicitly if no_ack = false
        if !self.no_ack {
            let mut args = BasicAckArguments::new();
            args.delivery_tag = deliver.delivery_tag();
            channel.basic_ack(args).await.unwrap();
        }
    }
}
