use crate::frame::{Close, CloseChannel};

use super::{channel::Channel, connection::Connection, error::Error};
use async_trait::async_trait;
use tracing::error;

type Result<T> = std::result::Result<T, Error>;

/////////////////////////////////////////////////////////////////////////////
#[async_trait]
pub trait ConnectionCallback {
    async fn close(&mut self, connection: &Connection, close: Close) -> Result<()>;
}
pub struct DefaultConnectionCallback;

#[async_trait]
impl ConnectionCallback for DefaultConnectionCallback {
    async fn close(&mut self, _connection: &Connection, close: Close) -> Result<()> {
        error!("{}", close);
        Ok(())
    }
}

/////////////////////////////////////////////////////////////////////////////
#[async_trait]
pub trait ChannelCallback {
    async fn close(&self, channel: &Channel, close: CloseChannel) -> Result<()>;
}

pub struct DefaultChannelCallback;

#[async_trait]
impl ChannelCallback for DefaultChannelCallback {
    async fn close(&self, _channel: &Channel, close: CloseChannel) -> Result<()> {
        error!("{}", close);
        Ok(())
    }
}
