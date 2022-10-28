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

pub(crate) struct ChannelResource {
    pub responder: Sender<IncomingMessage>,
    /// connection's default channel does not have dispatcher
    pub dispatcher: Option<Sender<Frame>>,
}
pub(crate) struct RegisterChannelResource {
    pub channel_id: Option<AmqpChannelId>,
    pub acker: oneshot::Sender<Option<AmqpChannelId>>,
    pub resource: ChannelResource,
}

pub(crate) enum ManagementCommand {
    RegisterChannelResource(RegisterChannelResource),
}
