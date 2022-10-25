mod split_connection;
mod connection_manager;
mod reader_handler;
mod writer_handler;
mod channel_id_repo;
mod error;

pub use split_connection::*;
pub use connection_manager::*;
pub use error::*;

/////////////////////////////////////////////////////////////////////////////
use tokio::sync::{mpsc::Sender, oneshot};
use crate::frame::Frame;
use amqp_serde::types::{AmqpChannelId, AmqpReplyCode};

pub type OutgoingMessage = (AmqpChannelId, Frame);

// TODO: move definition to receiver side, a.k.a API layer
#[derive(Debug)]
pub(crate) enum IncomingMessage {
    Ok(Frame),
    Exception(AmqpReplyCode, String),
}

pub(crate) struct ConsumerMessage;

pub(crate) struct RegisterResponder {
    pub channel_id: Option<AmqpChannelId>,
    pub responder: Sender<IncomingMessage>,
    pub acker: oneshot::Sender<Option<AmqpChannelId>>,
}

pub(crate) struct RegisterConsumer {
    pub channel_id: AmqpChannelId,
    pub consumer: Sender<ConsumerMessage>,
    pub acker: oneshot::Sender<()>,
}

pub(crate) enum ManagementCommand {
    RegisterResponder(RegisterResponder),
    RegisterConsumer(RegisterConsumer),
}

pub(crate) struct InternalChannels {
    /// The sender half to forward outgoing message to `WriterHandler`
    pub outgoing_tx: Sender<OutgoingMessage>,
    /// The sender half to send management commands to  `ReaderHandler`
    pub mgmt_tx: Sender<ManagementCommand>,
}