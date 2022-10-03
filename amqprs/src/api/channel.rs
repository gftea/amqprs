use amqp_serde::types::AmqpChannelId;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    frame::{CloseChannel, Declare, Frame},
    net::Message,
};

use super::error::Error;

pub struct Channel {
    channel_id: AmqpChannelId,
    tx: Sender<Message>,
    rx: Receiver<Frame>,
}

type Result<T> = std::result::Result<T, Error>;

impl Channel {
    /// new channel can only be created by Connection type
    pub(crate) fn new(channel_id: AmqpChannelId, tx: Sender<Message>, rx: Receiver<Frame>) -> Self {
        Self { channel_id, tx, rx }
    }
    pub async fn exchange_declare(&mut self) -> Result<()> {
        let mut declare = Declare::default();
        declare.set_passive();
        self.tx
            .send((self.channel_id, declare.into_frame()))
            .await?;
        match self.rx.recv().await.ok_or_else(|| Error::ChannelUseError)? {
            Frame::DeclareOk(_, _) => Ok(()),
            _ => Err(Error::ChannelUseError),
        }
    }

    pub async fn close(mut self) -> Result<()> {
        self.tx
            .send((self.channel_id, CloseChannel::default().into_frame()))
            .await?;
        match self
            .rx
            .recv()
            .await
            .ok_or_else(|| Error::ChannelCloseError)?
        {
            Frame::CloseChannelOk(_, _) => Ok(()),
            _ => Err(Error::ChannelCloseError),
        }
    }
}
