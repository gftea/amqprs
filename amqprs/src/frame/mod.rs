use amqp_serde::{
    from_bytes,
    types::{AmqpChannelId, LongUint, Octect, ShortUint},
};
use serde::{Deserialize, Serialize};
use std::fmt;

////////////////////////////////////////////////////////////////////////
// macros should appear before module declaration
#[macro_use]
mod helpers {
    // common interfaces of each method type
    macro_rules! impl_method_frame {
        ($name:ident, $class_id:literal, $method_id:literal) => {
            impl $name {
                pub fn header() -> &'static MethodHeader {
                    static __METHOD_HEADER: MethodHeader = MethodHeader::new($class_id, $method_id);
                    &__METHOD_HEADER
                }
                pub fn into_frame(self) -> Frame {
                    Frame::$name(Self::header(), self)
                }
            }
        };
    }

    macro_rules! impl_frame {
    ($($class_id:literal => $($method_id:literal : $method:ident),+);+) => {
        // function to decode method frame
        fn decode_method_frame(header: MethodHeader, content: &[u8]) -> Result<Frame, Error> {
            match header.class_id() {
                $($class_id => {
                    match header.method_id() {
                        $($method_id => Ok(from_bytes::<$method>(content)?.into_frame()),)+
                        _ => unimplemented!("unknown method id"),
                    }
                })+
                _ => unimplemented!("unknown class id"),
            }
        }

        // common interfaces of each method type
        $($(impl_method_frame!{$method, $class_id, $method_id})+)+

        // `Frame` enum to generailize various frames.
        // To avoid generic type parameter for new type depends on `Frame`.
        // Only wrap the frame payload in enum variant, excluding the `FrameHeader` and FRAME_END byte
        // The `Frame` type only need to implement Serialize, because when decoding a `Frame`,
        // `FrameHeader`, its payload, and `FRAME_END` bytes are desrialized separately
        #[derive(Debug, Serialize)]
        #[serde(untagged)]
        pub enum Frame {
            // method frame payload = method header + method
            $($($method(&'static MethodHeader, $method),)+)+

            HeartBeat(HeartBeat),
            ContentHeader(ContentHeader),
            ContentBody(ContentBody),
        }
    };

}
}
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

/////////////////////////////////////////////////////////////////
impl_frame! {
    // == Connection ==
    10 =>   10: Start,
            11: StartOk,
            20: Secure,
            21: SecureOk,
            30: Tune,
            31: TuneOk,
            40: Open,
            41: OpenOk,
            50: Close,
            51: CloseOk,
            60: Blocked,
            61: Unblocked,
            70: UpdateSecret,
            71: UpdateSecretOk;
    // == Channel ==
    20 =>   10: OpenChannel,
            11: OpenChannelOk,
            20: Flow,
            21: FlowOk,
            40: CloseChannel,
            41: CloseChannelOk;
    // == Access == Deprecated: https://www.rabbitmq.com/spec-differences.html
    30 =>   10: Request,
            11: RequestOk;
    // == Exchange ==
    40 =>   10: Declare,
            11: DeclareOk,
            20: Delete,
            21: DeleteOk,
            30: Bind,
            31: BindOk,
            40: Unbind,
            51: UnbindOk;
    // == Queue ==
    50 =>   10: DeclareQueue,
            11: DeclareQueueOk,
            20: BindQueue,
            21: BindQueueOk,
            30: PurgeQueue,
            31: PurgeQueueOk,
            40: DeleteQueue,
            41: DeleteQueueOk,
            50: UnbindQueue,
            51: UnbindQueueOk;
    // == Basic ==
    60 =>   10: Qos,
            11: QosOk,
            20: Consume,
            21: ConsumeOk,
            30: Cancel,
            31: CancelOk,
            40: Publish,
            50: Return,
            60: Deliver,
            70: Get,
            71: GetOk,
            72: GetEmpty,
            80: Ack,
            90: Reject,
            100: RecoverAsync,
            110: Recover,
            111: RecoverOk,
            120: Nack;
    // == Confirm ==
    85 =>   10: Select,
            11: SelectOk;
    // == Transaction ==
    90 =>   10: SelectTx,
            11: SelectTxOk,
            20: Commit,
            21: CommitOk,
            30: Rollback,
            31: RollbackOk
}

//////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FrameHeader {
    pub frame_type: Octect, // 1: method, 2: content-header, 3: content-body, 8: heartbeat
    pub channel: ShortUint,
    pub payload_size: LongUint,
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Frame {
    pub fn get_frame_type(&self) -> Octect {
        match self {
            Frame::HeartBeat(_) => FRAME_HEARTBEAT,
            Frame::ContentHeader(_) => FRAME_CONTENT_HEADER,
            Frame::ContentBody(_) => FRAME_CONTENT_BODY,
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
                let method_raw = match buf.get(FRAME_HEADER_SIZE + 4..total_size as usize - 1) {
                    Some(s) => s,
                    None => unreachable!("out of bound"),
                };

                let frame = decode_method_frame(header, method_raw)?;

                Ok(Some((total_size, channel, frame)))
            }
            FRAME_HEARTBEAT => Ok(Some((total_size, channel, Frame::HeartBeat(HeartBeat)))),
            FRAME_CONTENT_HEADER => {
                let mut start = FRAME_HEADER_SIZE;
                let mut end = start + 12;
                let header_common: ContentHeaderCommon = from_bytes(match buf.get(start..end) {
                    Some(s) => s,
                    None => unreachable!("out of bound"),
                })?;

                start = end;
                end = total_size as usize - 1;
                let basic_properties: BasicProperties = from_bytes(match buf.get(start..end) {
                    Some(s) => s,
                    None => unreachable!("out of bound"),
                })?;

                Ok(Some((
                    total_size,
                    channel,
                    Frame::ContentHeader(ContentHeader::new(header_common, basic_properties)),
                )))
            }
            FRAME_CONTENT_BODY => {
                let start = FRAME_HEADER_SIZE;
                let end = total_size as usize - 1;
                let body = buf.get(start..end).expect("should never fail");
                Ok(Some((
                    total_size,
                    channel,
                    Frame::ContentBody(ContentBody::new(body.to_vec())),
                )))
            }
            _ => Err(Error::Corrupted),
        }
    }
}

/////////////////////////////////////////////////////////////////////////////
