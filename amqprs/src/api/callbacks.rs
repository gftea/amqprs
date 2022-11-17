use crate::frame::{Close, CloseChannel};

use super::{channel::Channel, connection::Connection};
use async_trait::async_trait;
use tracing::error;

/////////////////////////////////////////////////////////////////////////////
#[async_trait]
pub trait ConnectionCallback {
    async fn close(&mut self, connection: &Connection, close: Close);
}
pub struct DefaultConnectionCallback;

#[async_trait]
impl ConnectionCallback for DefaultConnectionCallback {
    async fn close(&mut self, _connection: &Connection, close: Close) {
        error!("{}", close);
    }
}

/////////////////////////////////////////////////////////////////////////////
#[async_trait]
pub trait ChannelCallback {
    async fn close(&mut self, channel: &Channel, close: CloseChannel);
}

pub struct DefaultChannelCallback;

#[async_trait]
impl ChannelCallback for DefaultChannelCallback {
    async fn close(&mut self, _channel: &Channel, close: CloseChannel) {
        error!("{}", close);
    }
}
