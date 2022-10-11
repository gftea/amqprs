use amqp_serde::types::LongUint;

use crate::{
    api::error::Error,
    frame::{Qos, Frame},
    net::Response,
};

use super::{Channel, Result, ServerSpecificArguments};

#[derive(Debug, Clone)]
pub struct BasicQosArguments {
    pub prefetch_size: u32,
    pub prefetch_count: u16,
    pub global: bool,

}

impl BasicQosArguments {
    pub fn new() -> Self {
        Self {
            prefetch_size: 0,
            prefetch_count: 0,
            global: false,
        }
    }
}


/////////////////////////////////////////////////////////////////////////////
impl Channel {
    pub async fn basic_qos(&mut self, args: BasicQosArguments) -> Result<()> {
        let qos = Qos { prefetch_size: args.prefetch_size, prefetch_count: args.prefetch_count, global: args.global };
        synchronous_request!(
            self.tx,
            (self.channel_id, qos.into_frame()),
            self.rx,
            Frame::QosOk,
            (),
            Error::ChannelUseError
        )
    }
}