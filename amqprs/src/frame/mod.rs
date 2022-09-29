use amqp_serde::{
    constants::{
        FRAME_BODY, FRAME_END, FRAME_HEADER, FRAME_HEADER_SIZE, FRAME_HEARTBEAT, FRAME_METHOD,
    },
    from_bytes,
    types::{LongUint, Octect, ShortUint},
};

use serde::{Deserialize, Serialize};

///////////////////////////////////////////////////////////
mod content_body;
mod content_header;
mod error;
mod heartbeat;
mod method;
mod protocol_header;

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

    Declare(&'static MethodHeader, Declare),
    #[serde(skip_serializing)]
    DeclareOk(&'static MethodHeader, DeclareOk),

    HeartBeat(HeartBeat),

    ContentHeader(ContentHeader),

    ContentBody(ContentBody),
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
    pub fn decode(buf: &[u8]) -> Result<(usize, ShortUint, Frame), Error> {
        // check frame header, 7 octects
        if buf.len() < FRAME_HEADER_SIZE {
            return Err(Error::Incomplete);
        }

        let FrameHeader {
            frame_type,
            channel,
            payload_size,
        } = from_bytes(match buf.get(0..FRAME_HEADER_SIZE) {
            Some(s) => s,
            None => unreachable!(),
        })?;

        // check full frame is received payload_size + 8 octects
        let total_size = payload_size as usize + FRAME_HEADER_SIZE + 1;
        if total_size > buf.len() {
            return Err(Error::Incomplete);
        }
        // check frame end
        match buf.get(total_size - 1) {
            Some(v) => {
                // expect frame_end
                if &FRAME_END != v {
                    return Err(Error::Corrupted);
                }
            }
            None => unreachable!(),
        };

        // parse frame payload
        match frame_type {
            FRAME_METHOD => {
                let header: MethodHeader =
                    from_bytes(match buf.get(FRAME_HEADER_SIZE..FRAME_HEADER_SIZE + 4) {
                        Some(s) => s,
                        None => unreachable!(),
                    })?;
                let content = match buf.get(FRAME_HEADER_SIZE + 4..total_size as usize - 1) {
                    Some(s) => s,
                    None => unreachable!(),
                };

                let frame = if &header == Start::header() {
                    from_bytes::<Start>(content)?.into_frame()
                } else if &header == Tune::header() {
                    from_bytes::<Tune>(content)?.into_frame()
                } else if &header == OpenOk::header() {
                    from_bytes::<OpenOk>(content)?.into_frame()
                } else if &header == Close::header() {
                    from_bytes::<Close>(content)?.into_frame()
                } else if &header == CloseOk::header() {
                    from_bytes::<CloseOk>(content)?.into_frame()
                } else if &header == OpenChannelOk::header() {
                    from_bytes::<OpenChannelOk>(content)?.into_frame()
                } else if &header == DeclareOk::header() {
                    from_bytes::<DeclareOk>(content)?.into_frame()
                } else {
                    println!("header: {:?}", header);
                    println!("content: {:?}", content);
                    unreachable!("unknown frame");
                };
                Ok((total_size, channel, frame))
            }
            FRAME_HEARTBEAT | FRAME_HEADER | FRAME_BODY => todo!(),
            _ => Err(Error::Corrupted),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
