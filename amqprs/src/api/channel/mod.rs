//! API implementation of AMQP Channel
//!

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use amqp_serde::types::{AmqpChannelId, FieldTable, FieldValue};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot, RwLock,
};

use crate::{
    api::error::Error,
    frame::{CloseChannel, Flow, Frame},
    net::{IncomingMessage, ManagementCommand, OutgoingMessage, RegisterChannelResource},
};

type Result<T> = std::result::Result<T, Error>;

/// Represent an AMQP Channel.
///
/// To create a AMQP channel, use [`Connection::channel` method][`channel`]
///
/// [`channel`]: crate::api::connection::Connection::channel
pub struct Channel {
    pub(in crate::api) is_open: bool,
    pub(in crate::api) channel_id: AmqpChannelId,
    pub(in crate::api) outgoing_tx: Sender<OutgoingMessage>,
    pub(in crate::api) incoming_rx: Receiver<IncomingMessage>,
    pub(in crate::api) mgmt_tx: Sender<ManagementCommand>,

    /// callback queue
    pub(in crate::api) consumer_queue: SharedConsumerQueue,
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    ///
    pub async fn flow(&mut self, active: bool) -> Result<()> {
        synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, Flow { active }.into_frame()),
            self.incoming_rx,
            Frame::FlowOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    /// User must close the channel to avoid channel leak
    pub async fn close(mut self) -> Result<()> {
        synchronous_request!(
            self.outgoing_tx,
            (self.channel_id, CloseChannel::default().into_frame()),
            self.incoming_rx,
            Frame::CloseChannelOk,
            Error::ChannelCloseError
        )?;
        self.is_open = false;
        Ok(())
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if self.is_open {
            let tx = self.outgoing_tx.clone();
            let channel_id = self.channel_id;
            // When a Channel drop, it should notify server to close it to avoid channel leak.
            tokio::spawn(async move {
                tx.send((channel_id, CloseChannel::default().into_frame()))
                    .await
                    .expect("CloseChannel when drop to avoid leak of channel");
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
    fn into_field_table(self) -> FieldTable {
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

use super::{consumer::Consumer, connection::SharedConsumerQueue};
