//! API implementation of AMQP Channel
//!

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use amqp_serde::types::{AmqpChannelId, FieldTable, FieldValue};
use tokio::sync::{mpsc, oneshot};

use super::callbacks::ChannelCallback;
use crate::{
    api::{error::Error, Result},
    frame::{CloseChannel, CloseChannelOk, Deliver, Flow, FlowOk, Frame, MethodHeader, Return},
    net::{ConnManagementCommand, IncomingMessage, OutgoingMessage},
    BasicProperties,
};
use tracing::{error, trace};

pub(crate) const CONSUMER_MESSAGE_BUFFER_SIZE: usize = 32;

#[derive(Debug)]
pub(crate) struct ConsumerMessage {
    deliver: Option<Deliver>,
    basic_properties: Option<BasicProperties>,
    content: Option<Vec<u8>>,
}
pub(crate) struct ReturnMessage {
    ret: Option<Return>,
    basic_properties: Option<BasicProperties>,
}

pub(crate) struct RegisterContentConsumer {
    consumer_tag: String,
    consumer_tx: mpsc::Sender<ConsumerMessage>,
}
pub(crate) struct UnregisterContentConsumer {
    consumer_tag: String,
}

pub(crate) struct RegisterGetContentResponder {
    tx: mpsc::Sender<IncomingMessage>,
}

pub(crate) struct RegisterOneshotResponder {
    pub method_header: &'static MethodHeader,
    pub responder: oneshot::Sender<IncomingMessage>,
    pub acker: oneshot::Sender<()>,
}

pub(crate) struct RegisterChannelCallback {
    pub callback: Box<dyn ChannelCallback + Send + 'static>,
}
pub(crate) enum DispatcherManagementCommand {
    RegisterContentConsumer(RegisterContentConsumer),
    UnregisterContentConsumer(UnregisterContentConsumer),
    RegisterGetContentResponder(RegisterGetContentResponder),
    RegisterOneshotResponder(RegisterOneshotResponder),
    RegisterChannelCallback(RegisterChannelCallback),
}

/// Represent an AMQP Channel.
///
/// To create a AMQP channel, use [`Connection::channel` method][`channel`]
///
/// [`channel`]: crate::api::connection::Connection::channel
#[derive(Debug, Clone)]
pub struct Channel {
    shared: Arc<SharedChannelInner>,
}

#[derive(Debug)]
pub(crate) struct SharedChannelInner {
    is_open: AtomicBool,

    channel_id: AmqpChannelId,

    outgoing_tx: mpsc::Sender<OutgoingMessage>,

    conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,

    dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
}

impl SharedChannelInner {
    pub(in crate::api) fn new(
        is_open: AtomicBool,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            is_open,
            channel_id,
            outgoing_tx,
            conn_mgmt_tx,
            dispatcher_mgmt_tx,
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    pub(in crate::api) fn new(
        is_open: AtomicBool,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            shared: Arc::new(SharedChannelInner::new(
                is_open,
                channel_id,
                outgoing_tx,
                conn_mgmt_tx,
                dispatcher_mgmt_tx,
            )),
        }
    }

    pub async fn register_callback<F>(&self, callback: F) -> Result<()>
    where
        F: ChannelCallback + Send + 'static,
    {
        let cmd = RegisterChannelCallback {
            callback: Box::new(callback),
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterChannelCallback(cmd))
            .await?;
        Ok(())
    }
    async fn register_responder(
        &self,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<IncomingMessage>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterOneshotResponder {
            method_header,
            responder,
            acker,
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterOneshotResponder(cmd))
            .await?;
        acker_rx.await?;
        Ok(responder_rx)
    }
    pub fn is_connection_closed(&self) -> bool {
        self.shared.conn_mgmt_tx.is_closed()
    }

    pub fn channel_id(&self) -> AmqpChannelId {
        self.shared.channel_id
    }
    pub(crate) fn set_is_open(&self, is_open: bool) {
        self.shared.is_open.store(is_open, Ordering::Relaxed);
    }
    pub fn is_open(&self) -> bool {
        self.shared.is_open.load(Ordering::Relaxed)
    }

    /// asks the peer to pause or restart the flow of content data
    /// `true` means the peer will start sending or continue to send content frames; `false` means it will not.
    pub async fn flow(&mut self, active: bool) -> Result<bool> {
        let responder_rx = self.register_responder(FlowOk::header()).await?;
        let flow_ok = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, Flow::new(active).into_frame()),
            responder_rx,
            Frame::FlowOk,
            Error::ChannelUseError
        )?;
        Ok(flow_ok.active())
    }

    /// User must close the channel to avoid channel leak
    pub async fn close(self) -> Result<()> {
        self.set_is_open(false);

        // if connection has been closed, no need to close channel
        if !self.is_connection_closed() {
            let responder_rx = self.register_responder(CloseChannelOk::header()).await?;

            synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, CloseChannel::default().into_frame()),
                responder_rx,
                Frame::CloseChannelOk,
                Error::ChannelCloseError
            )?;
        }

        Ok(())
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if let Ok(true) =
            self.shared
                .is_open
                .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
        {
            trace!("drop and close channel {}.", self.channel_id());

            let channel = self.clone();
            tokio::spawn(async move {
                if let Err(err) = channel.close().await {
                    error!(
                        "error occurred during close channel when drop, cause: {}",
                        err
                    );
                }
            });
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
/// A table map of arguments
/// The syntax and semantics of these arguments depends on the server implementation.
#[derive(Debug, Clone)]
pub struct TableArguments {
    table: FieldTable,
}

impl TableArguments {
    pub fn new() -> Self {
        Self {
            table: FieldTable::new(),
        }
    }

    pub fn insert_str(&mut self, key: String, value: &str) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::S(value.try_into().unwrap()),
        );
    }
    pub fn insert_bool(&mut self, key: String, value: bool) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::t(value.try_into().unwrap()),
        );
    }
    pub fn insert_u8(&mut self, key: String, value: u8) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::B(value.try_into().unwrap()),
        );
    }
    pub fn insert_u16(&mut self, key: String, value: u8) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::u(value.try_into().unwrap()),
        );
    }
    pub fn insert_u32(&mut self, key: String, value: u32) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::i(value.try_into().unwrap()),
        );
    }
    pub fn insert_i64(&mut self, key: String, value: i64) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::l(value.try_into().unwrap()),
        );
    }
    /// the field table type should be hidden from API
    pub(crate) fn into_field_table(self) -> FieldTable {
        self.table
    }
}
/////////////////////////////////////////////////////////////////////////////
mod dispatcher;
pub(crate) use dispatcher::*;

mod basic;
mod confim;
mod exchange;
mod queue;
mod tx;

pub use basic::*;
pub use exchange::*;
pub use queue::*;
