//! API implementation of AMQP Channel
//!

use amqp_serde::types::{AmqpChannelId, FieldTable, FieldValue};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    api::error::Error,
    frame::{CloseChannel, Frame, Flow},
    net::{Request, Response},
};

type Result<T> = std::result::Result<T, Error>;

/// Represent an AMQP Channel.
///
/// To create a AMQP channel, use [`Connection::channel` method][`channel`]
///
/// [`channel`]: crate::api::connection::Connection::channel
pub struct Channel {
    channel_id: AmqpChannelId,
    tx: Sender<Request>,
    rx: Receiver<Response>,
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    /// New channel can only be created by [`channel`]
    pub(crate) fn new(
        channel_id: AmqpChannelId,
        tx: Sender<Request>,
        rx: Receiver<Response>,
    ) -> Self {
        Self { channel_id, tx, rx }
    }
    pub async fn flow(&mut self, active: bool) -> Result<()> {
        synchronous_request!(
            self.tx,
            (self.channel_id, Flow { active }.into_frame()),
            self.rx,
            Frame::FlowOk,
            (),
            Error::ChannelUseError
        )
    }
    pub async fn close(mut self) -> Result<()> {
        synchronous_request!(
            self.tx,
            (self.channel_id, CloseChannel::default().into_frame()),
            self.rx,
            Frame::CloseChannelOk,
            (),
            Error::ChannelCloseError
        )
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        let tx = self.tx.clone();
        let channel_id = self.channel_id;
        // When a Channel drop, it should notify server to close it to avoid channel leak.
        tokio::spawn(async move {
            tx.send((channel_id, CloseChannel::default().into_frame()))
                .await
                .unwrap();
        });
    }
}

/// A set of arguments for the declaration.
/// The syntax and semantics of these arguments depends on the server implementation.
#[derive(Debug, Clone)]
pub struct ServerSpecificArguments {
    table: FieldTable
}

impl ServerSpecificArguments {
    pub fn new() -> Self {
        Self {
            table: FieldTable::new(),
        }
    }
    
    pub fn insert_str(&mut self, key: String, value: &str ) {
        self.table.insert(key.try_into().unwrap(), FieldValue::S(value.try_into().unwrap()));
    }
    pub fn insert_bool(&mut self, key: String, value: bool ) {
        self.table.insert(key.try_into().unwrap(), FieldValue::t(value.try_into().unwrap()));
    }   
    pub fn insert_u8(&mut self, key: String, value: u8 ) {
        self.table.insert(key.try_into().unwrap(), FieldValue::B(value.try_into().unwrap()));
    }
    pub fn insert_u16(&mut self, key: String, value: u8 ) {
        self.table.insert(key.try_into().unwrap(), FieldValue::u(value.try_into().unwrap()));
    }    
    pub fn insert_u32(&mut self, key: String, value: u32 ) {
        self.table.insert(key.try_into().unwrap(), FieldValue::i(value.try_into().unwrap()));
    }    
    pub fn insert_i64(&mut self, key: String, value: i64 ) {
        self.table.insert(key.try_into().unwrap(), FieldValue::l(value.try_into().unwrap()));
    }        
    // the field table type should be hidden from API
    fn into_field_table(self) -> FieldTable {
        self.table
    }
}
/////////////////////////////////////////////////////////////////////////////
mod exchange;
mod queue;
mod basic;

pub use exchange::*;
pub use queue::*;
pub use basic::*;
