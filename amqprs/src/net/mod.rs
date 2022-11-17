mod channel_id_repo;
mod channel_manager;
mod error;
mod reader_handler;
mod split_connection;
mod writer_handler;



pub(crate) use error::*;
pub(crate) use reader_handler::*;
pub(crate) use split_connection::*;
pub(crate) use writer_handler::*;
pub(crate) use channel_manager::*;
/////////////////////////////////////////////////////////////////////////////
use crate::{
    api::{callbacks::{ChannelCallback, ConnectionCallback}, channel::Channel},
    frame::{Frame, MethodHeader},
};
use amqp_serde::types::AmqpChannelId;
use tokio::sync::{mpsc::Sender, oneshot};

pub type OutgoingMessage = (AmqpChannelId, Frame);

pub(crate) type IncomingMessage = Frame;


pub(crate) struct RegisterChannelResource {
    /// If None, `net` handler will allocate a channel id for client
    pub channel_id: Option<AmqpChannelId>,
    /// send `None` to client if `net` handler fail to allocate a channel id
    pub acker: oneshot::Sender<Option<AmqpChannelId>>,
    pub resource: ChannelResource,
}

pub(crate) struct RegisterResponder {
    pub channel_id: AmqpChannelId,
    pub method_header: &'static MethodHeader,
    pub responder: oneshot::Sender<IncomingMessage>,
    pub acker: oneshot::Sender<()>,
}

pub(crate) struct RegisterConnectionCallback {
    pub callback: Box<dyn ConnectionCallback + Send + 'static>,
}

pub(crate) enum ConnManagementCommand {
    RegisterChannelResource(RegisterChannelResource),
    RegisterResponder(RegisterResponder),
    RegisterConnectionCallback(RegisterConnectionCallback),
}
