//! API implementation of AMQP Channel
//!

use amqp_serde::types::AmqpChannelId;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    api::error::Error,
    frame::{CloseChannel, Frame, Flow},
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
    pub(crate) fn new(
        channel_id: AmqpChannelId,
        tx: Sender<Request>,
        rx: Receiver<Response>,
    ) -> Self {
        Self { channel_id, tx, rx }
    }
    pub async fn flow(&mut self, active: bool) -> Result<()> {
        synchronous_request!(
            self.tx,
            (self.channel_id, Flow { active }.into_frame()),
            self.rx,
            Frame::FlowOk,
            (),
            Error::ChannelUseError
        )
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

impl Drop for Channel {
    fn drop(&mut self) {
        let tx = self.tx.clone();
        let channel_id = self.channel_id;
        // When a Channel drop, it should notify server to close it to avoid channel leak.
        tokio::spawn(async move {
            tx.send((channel_id, CloseChannel::default().into_frame()))
                .await
                .unwrap();
        });
    }
}
/////////////////////////////////////////////////////////////////////////////
mod exchange;
pub use exchange::*;
