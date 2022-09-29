use amqp_serde::types::{ShortStr, LongStr};
use serde::{Serialize, Deserialize};

use super::impl_mapping;
use crate::frame::{Frame, MethodHeader};

impl_mapping!(OpenChannel, 20, 10);
impl_mapping!(OpenChannelOk, 20, 11);


#[derive(Debug, Serialize)]
pub struct OpenChannel {
    out_of_band: ShortStr,

}
impl Default for OpenChannel {
    fn default() -> Self {
        Self { out_of_band: "".try_into().unwrap() }
    }
}

#[derive(Debug, Deserialize)]
pub struct OpenChannelOk {
    channel_id: LongStr,
}
