mod channel_id_repo;
mod connection_manager;
mod channel_manager;
mod error;
mod reader_handler;
mod split_connection;
mod writer_handler;

use std::collections::HashMap;

pub(crate) use connection_manager::*;
pub(crate) use error::*;
pub(crate) use split_connection::*;
pub(crate) use reader_handler::*;
pub(crate) use writer_handler::*;

/////////////////////////////////////////////////////////////////////////////
use crate::{frame::{Frame, MethodHeader}, api::callbacks::{ConnectionCallback, ChannelCallback}};
use amqp_serde::types::AmqpChannelId;
use tokio::sync::{mpsc::Sender, oneshot};

pub type OutgoingMessage = (AmqpChannelId, Frame);

pub(crate) type IncomingMessage = Frame;

pub(crate) struct ChannelResource {
    /// responder to acknowledge synchronous request
    pub responders: HashMap<&'static MethodHeader, oneshot::Sender<IncomingMessage>>,
    /// connection's default channel does not have dispatcher
    pub dispatcher: Option<Sender<IncomingMessage>>,
}
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
pub(crate) struct RegisterChannelCallback {
    pub callback: Box<dyn ChannelCallback + Send + 'static>,
}
pub(crate) enum ConnManagementCommand {
    RegisterChannelResource(RegisterChannelResource),
    RegisterResponder(RegisterResponder),
    RegisterConnectionCallback(RegisterConnectionCallback),
}
