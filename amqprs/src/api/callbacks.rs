use std::str::from_utf8;

use super::{channel::Channel, connection::Connection};
use crate::api::Result;
use crate::frame::Cancel;
use crate::{
    frame::{Ack, Blocked, Close, CloseChannel, Flow, Nack, Return, Unblocked},
    BasicProperties,
};
use async_trait::async_trait;
use tracing::{error, info};

/////////////////////////////////////////////////////////////////////////////
#[async_trait]
pub trait ConnectionCallback {
    async fn close(&mut self, connection: &Connection, close: Close) -> Result<()>;
    async fn blocked(&mut self, connection: &Connection, blocked: Blocked);
    async fn unblocked(&mut self, connection: &Connection, blocked: Unblocked);
}
pub struct DefaultConnectionCallback;

#[async_trait]
impl ConnectionCallback for DefaultConnectionCallback {
    async fn close(&mut self, _connection: &Connection, close: Close) -> Result<()> {
        error!("{}!", close);
        Ok(())
    }

    async fn blocked(&mut self, _connection: &Connection, blocked: Blocked) {
        info!("connection blocked by server, reason: {}.", blocked.reason());
    }
    async fn unblocked(&mut self, _connection: &Connection, _blocked: Unblocked) {
        info!("connection unblocked by server.");
    }
}

/////////////////////////////////////////////////////////////////////////////
#[async_trait]
pub trait ChannelCallback {
    async fn close(&mut self, channel: &Channel, close: CloseChannel) -> Result<()>;
    async fn cancel(&mut self, channel: &Channel,  cancel: Cancel) -> Result<()>;

    async fn flow(&mut self, channel: &Channel, flow: Flow) -> Result<bool>;
    async fn publish_ack(&mut self, channel: &Channel, ack: Ack);
    async fn publish_nack(&mut self, channel: &Channel, nack: Nack);
    async fn publish_return(
        &mut self,
        channel: &Channel,
        ret: Return,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    );
}

pub struct DefaultChannelCallback;

#[async_trait]
impl ChannelCallback for DefaultChannelCallback {
    async fn close(&mut self, _channel: &Channel, close: CloseChannel) -> Result<()> {
        error!("{}!", close);
        Ok(())
    }
    async fn cancel(&mut self, _channel: &Channel, cancel: Cancel) -> Result<()> {
        info!("receive cancel for consumer: {}.", cancel.consumer_tag());
        Ok(())
    }    
    async fn flow(&mut self, channel: &Channel, flow: Flow) -> Result<bool> {
        info!("channel flow request from server, {}.", flow.active());
        Ok(true)
    }
    async fn publish_ack(&mut self, channel: &Channel, ack: Ack) {
        info!("channel publish ack from server, {}.", ack.delivery_tag());
    }
    async fn publish_nack(&mut self, channel: &Channel, nack: Nack) {
        info!("channel publish nack from server, {}.", nack.delivery_tag());
    }
    async fn publish_return(
        &mut self,
        channel: &Channel,
        ret: Return,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        info!(">>>>> Publish Return Start <<<<<.");
        info!("{}.", ret);
        info!("{}.", basic_properties,);
        info!("{}.", from_utf8(&content).unwrap());
        info!(">>>>> Publish Return End <<<<<.");
    }
}
