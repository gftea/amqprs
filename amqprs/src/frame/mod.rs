use amqp_serde::{
    from_bytes,
    types::{AmqpChannelId, LongUint, Octect, ShortUint},
};

use serde::{Deserialize, Serialize};

///////////////////////////////////////////////////////////
mod constants;
mod content_body;
mod content_header;
mod error;
mod heartbeat;
mod method;
mod protocol_header;

pub use constants::*;
pub use content_body::*;
pub use content_header::*;
pub use error::*;
pub use heartbeat::*;
pub use method::*;
pub use protocol_header::*;
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FrameHeader {
    pub frame_type: Octect, // 1: method, 2: content-header, 3: content-body, 8: heartbeat
    pub channel: ShortUint,
    pub payload_size: LongUint,
}

/// Frame type design choice:
/// Use enum to generailize the types instead of generic type parameter, which avoid generic type parameter in
/// interfaces or new type depends on Frame
/// Only wrap the payload  instead of complete frame struture, reduce the size of type but leaving
/// the responsibility of constructing a complete frame in other placaes

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Frame {
    #[serde(skip_serializing)]
    Start(&'static MethodHeader, Start),
    StartOk(&'static MethodHeader, StartOk),

    #[serde(skip_serializing)]
    Tune(&'static MethodHeader, Tune),
    TuneOk(&'static MethodHeader, TuneOk),

    Open(&'static MethodHeader, Open),
    #[serde(skip_serializing)]
    OpenOk(&'static MethodHeader, OpenOk),

    Close(&'static MethodHeader, Close),
    CloseOk(&'static MethodHeader, CloseOk),

    OpenChannel(&'static MethodHeader, OpenChannel),
    #[serde(skip_serializing)]
    OpenChannelOk(&'static MethodHeader, OpenChannelOk),
    Flow(&'static MethodHeader, Flow),
    FlowOk(&'static MethodHeader, FlowOk),
    CloseChannel(&'static MethodHeader, CloseChannel),
    CloseChannelOk(&'static MethodHeader, CloseChannelOk),

    Declare(&'static MethodHeader, Declare),
    #[serde(skip_serializing)]
    DeclareOk(&'static MethodHeader, DeclareOk),

    HeartBeat(HeartBeat),

    ContentHeader(ContentHeader),

    ContentBody(ContentBody),
}

macro_rules! decode_method_frame {
    ($header:ident, $content:ident, $method:ident, $($others:ident),*) => {
        if &$header == $method::header() {
            from_bytes::<$method>($content)?.into_frame()
        } $(else if &$header == $others::header() {
            from_bytes::<$others>($content)?.into_frame()
        })*
        else {
            println!("header: {:?}", $header);
            println!("content: {:?}", $content);
            unreachable!("unknown frame");
        }
    };
}

impl Frame {
    pub fn get_frame_type(&self) -> Octect {
        match self {
            Frame::HeartBeat(_) => FRAME_HEARTBEAT,
            Frame::ContentHeader(_) => FRAME_HEADER,
            Frame::ContentBody(_) => FRAME_BODY,
            _ => FRAME_METHOD,
        }
    }
    /// To support channels multiplex on one connection, need to populate the channel id
    /// to support update of read buffer cursor, need to populate the number of bytes are read
    /// Return :
    ///     (Number of bytes are read, Channel id, the Frame)
    pub fn decode(buf: &[u8]) -> Result<Option<(usize, AmqpChannelId, Frame)>, Error> {
        // check frame header, 7 octects
        if buf.len() < FRAME_HEADER_SIZE {
            return Ok(None);
        }

        let FrameHeader {
            frame_type,
            channel,
            payload_size,
        } = from_bytes(match buf.get(0..FRAME_HEADER_SIZE) {
            Some(s) => s,
            None => unreachable!("out of bound"),
        })?;

        // check full frame is received payload_size + 8 octects
        let total_size = payload_size as usize + FRAME_HEADER_SIZE + 1;
        if total_size > buf.len() {
            return Ok(None);
        }
        // check frame end
        match buf.get(total_size - 1) {
            Some(v) => {
                // expect frame_end
                if &FRAME_END != v {
                    return Err(Error::Corrupted);
                }
            }
            None => unreachable!("out of bound"),
        };

        // parse frame payload
        match frame_type {
            FRAME_METHOD => {
                let header: MethodHeader =
                    from_bytes(match buf.get(FRAME_HEADER_SIZE..FRAME_HEADER_SIZE + 4) {
                        Some(s) => s,
                        None => unreachable!("out of bound"),
                    })?;
                let content = match buf.get(FRAME_HEADER_SIZE + 4..total_size as usize - 1) {
                    Some(s) => s,
                    None => unreachable!("out of bound"),
                };

                let frame = decode_method_frame!(
                    header,
                    content,
                    Start,
                    Tune,
                    OpenOk,
                    Close,
                    CloseOk,
                    OpenChannelOk,
                    Flow,
                    FlowOk,
                    CloseChannel,
                    CloseChannelOk,
                    DeclareOk
                );

                Ok(Some((total_size, channel, frame)))
            }
            FRAME_HEARTBEAT => Ok(Some((total_size, channel, Frame::HeartBeat(HeartBeat)))),
            FRAME_HEADER | FRAME_BODY => todo!(),
            _ => Err(Error::Corrupted),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
