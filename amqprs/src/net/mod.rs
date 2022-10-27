mod channel_id_repo;
mod connection_manager;
mod error;
mod reader_handler;
mod split_connection;
mod writer_handler;

use std::pin::Pin;

pub use connection_manager::*;
pub use error::*;
pub use split_connection::*;

/////////////////////////////////////////////////////////////////////////////
use crate::{api::consumer::Consumer, frame::Frame};
use amqp_serde::types::{AmqpChannelId, AmqpReplyCode};
use tokio::sync::{mpsc::Sender, oneshot};

pub type OutgoingMessage = (AmqpChannelId, Frame);

// TODO: move definition to receiver side, a.k.a API layer
#[derive(Debug)]
pub(crate) enum IncomingMessage {
    Ok(Frame),
    Exception(AmqpReplyCode, String),
}

pub(crate) struct ConsumerResource {
    pub consumer_tx: Sender<Frame>,
}

pub(crate) struct RegisterResponder {
    pub channel_id: Option<AmqpChannelId>,
    pub responder: Sender<IncomingMessage>,
    pub acker: oneshot::Sender<Option<AmqpChannelId>>,
}
// TODO: remove, if we do not keep consumer tag in ReaderHandler,
// instead, we have consumer task spawned by AMQ channel type,
// we can merge this register command with RegisterResponder
pub(crate) struct RegisterConsumer {
    pub channel_id: AmqpChannelId,

    pub consumer_resource: ConsumerResource,
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
