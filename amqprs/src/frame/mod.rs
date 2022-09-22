use std::marker::PhantomData;

use amqp_serde::types::{Octect, ShortUint, LongUint};
use serde::{Serialize, Deserialize};

mod protocol_header;
pub use protocol_header::ProtocolHeader;

mod method;
pub use method::*;

mod content_header;
mod content_body;


#[derive(Debug, Serialize, Deserialize)]
pub struct Frame<T> {
    pub typ: Octect,
    pub channel: ShortUint,
    pub payload_size: LongUint,
    pub payload: T,
    pub frame_end: Octect,
}