use std::marker::PhantomData;

use amqp_serde::{
    constants::{FRAME_END, FRAME_METHOD},
    types::{LongUint, Octect, ShortUint},
};
use serde::{Deserialize, Serialize};

mod protocol_header;
pub use protocol_header::ProtocolHeader;

mod method;
pub use method::*;

use self::{
    content_body::ContentBodyPayload, content_header::ContentHeaderPayload, heartbeat::HeartBeat,
};
mod content_body;
mod content_header;
mod heartbeat;

#[derive(Debug, Serialize, Deserialize)]
pub struct FrameHeader {
    pub frame_type: Octect, // 1: method, 2: content-header, 3: content-body, 8: heartbeat
    pub channel: ShortUint,
    pub payload_size: LongUint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Frame<T> {
    pub header: FrameHeader,
    pub payload: T,
    frame_end: Octect,
}

impl<T> Frame<T>  {
    pub fn new_method(payload: T) -> Self {
        let header = FrameHeader {
            frame_type: FRAME_METHOD,
            channel: 0,
            payload_size: 0,
        };
        Self {
            header,
            payload,
            frame_end: FRAME_END,
        }
    }
    pub fn new_content_header() {}
    pub fn new_content_body() {}
    pub fn set_channel(&mut self, channel: ShortUint) {
        self.header.channel = channel;
    }
    pub fn set_payload_size(&mut self, payload_size: LongUint) {
        self.header.payload_size = payload_size;
    }
}
