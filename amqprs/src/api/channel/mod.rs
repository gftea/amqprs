//! API implementation of AMQP Channel
//! 

use amqp_serde::types::AmqpChannelId;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    api::error::Error,
    frame::{CloseChannel, Frame},
    net::{Request, Response},
};

type Result<T> = std::result::Result<T, Error>;

/// Represent an AMQP Channel.
/// 
/// To create a AMQP channel, use [`Connection::channel` method][`channel`]
/// 
/// [`channel`]: crate::api::connection::Connection::channel
pub struct Channel {
    channel_id: AmqpChannelId,
    tx: Sender<Request>,
    rx: Receiver<Response>,
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    /// New channel can only be created by [`channel`]
    pub(crate) fn new(channel_id: AmqpChannelId, tx: Sender<Request>, rx: Receiver<Response>) -> Self {
        Self { channel_id, tx, rx }
    }

    pub async fn close(mut self) -> Result<()> {
        synchronous_request!(
            self.tx,
            (self.channel_id, CloseChannel::default().into_frame()),
            self.rx,
            Frame::CloseChannelOk,
            (),
            Error::ChannelCloseError
        )
    }
}

/////////////////////////////////////////////////////////////////////////////
mod exchange;
pub use exchange::*;


