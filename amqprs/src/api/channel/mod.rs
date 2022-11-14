//! API implementation of AMQP Channel
//!

use std::collections::BTreeMap;

use amqp_serde::types::{AmqpChannelId, FieldTable, FieldValue};
use tokio::sync::{mpsc, oneshot};

use crate::{
    api::error::Error,
    frame::{CloseChannel, CloseChannelOk, Flow, FlowOk, Frame, MethodHeader},
    net::{ConnManagementCommand, IncomingMessage, OutgoingMessage, RegisterResponder},
};

type Result<T> = std::result::Result<T, Error>;

// pub struct Acker {
//     tx: mpsc::Sender<OutgoingMessage>,
//     channel_id: AmqpChannelId,
// }

/// Represent an AMQP Channel.
///
/// To create a AMQP channel, use [`Connection::channel` method][`channel`]
///
/// [`channel`]: crate::api::connection::Connection::channel
#[derive(Debug, Clone)]
pub struct Channel {
    pub(in crate::api) is_open: bool,
    pub(in crate::api) channel_id: AmqpChannelId,
    pub(in crate::api) outgoing_tx: mpsc::Sender<OutgoingMessage>,
    // pub(in crate::api) incoming_tx: Option<mpsc::Sender<IncomingMessage>>,
    // pub(in crate::api) incoming_rx: mpsc::Receiver<IncomingMessage>,
    pub(in crate::api) conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,

    // pub(in crate::api) dispatcher_rx: Option<mpsc::Receiver<Frame>>,
    pub(in crate::api) dispatcher_mgmt_tx: mpsc::Sender<DispatcherManagementCommand>,
    // pub(in crate::api) dispatcher_mgmt_rx: Option<mpsc::Receiver<DispatcherManagementCommand>>,
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    async fn register_responder(
        &self,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<Frame>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterResponder {
            channel_id: self.channel_id,
            method_header,
            responder,
            acker,
        };
        self.conn_mgmt_tx
            .send(ConnManagementCommand::RegisterResponder(cmd))
            .await?;
        acker_rx.await?;
        Ok(responder_rx)
    }

    ///
    pub async fn flow(&mut self, active: bool) -> Result<()> {
        let responder_rx = self.register_responder(FlowOk::header()).await?;
        synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, Flow { active }.into_frame()),
            responder_rx,
            Frame::FlowOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    /// User must close the channel to avoid channel leak
    pub async fn close(&mut self) -> Result<()> {
        let responder_rx = self.register_responder(CloseChannelOk::header()).await?;
        synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, CloseChannel::default().into_frame()),
            responder_rx,
            Frame::CloseChannelOk,
            Error::ChannelCloseError
        )?;
        self.is_open = false;
        Ok(())
    }

    pub fn is_connection_closed(&self) -> bool {
        self.conn_mgmt_tx.is_closed()
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if self.is_open {
            self.is_open = false;
            let mut channel = self.clone();
            tokio::spawn(async move {
                if let Err(err) = channel.close().await {
                    if !channel.is_connection_closed() {
                        panic!("failed to close channel when drop, cause: {}", err);
                    }
                }
            });
        }
    }
}

/// A set of arguments for the declaration.
/// The syntax and semantics of these arguments depends on the server implementation.
#[derive(Debug, Clone)]
pub struct ServerSpecificArguments {
    table: FieldTable,
}

impl ServerSpecificArguments {
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
    // the field table type should be hidden from API
    pub(crate) fn into_field_table(self) -> FieldTable {
        self.table
    }
}
/////////////////////////////////////////////////////////////////////////////
mod basic;
mod exchange;
mod queue;

pub use basic::*;
pub use exchange::*;
pub use queue::*;
