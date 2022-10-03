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
impl Channel {
    pub(crate) fn new(channel_id: AmqpChannelId, tx: Sender<Message>, rx: Receiver<Frame>) -> Self {
        Self { channel_id, tx, rx }
    }
    pub async fn exchange_declare(&mut self) -> Result<(), Error> {
        let mut declare = Declare::default();
        declare.set_passive();
        self.tx
            .send((self.channel_id, declare.into_frame()))
            .await?;
        match self.rx.recv().await {
            Some(frame) => match frame {
                Frame::DeclareOk(_, _) => Ok(()),
                _ => Err(Error::ChannelOpenFailure),
            },
            None => Err(Error::ChannelUseFailure),
        }
    }

    pub async fn close(mut self) {
        self.tx
            .send((self.channel_id, CloseChannel::default().into_frame()))
            .await
            .unwrap();
        self.rx.recv().await.unwrap();
        // TODO: how to remove the channel from channel manager
    }
}
