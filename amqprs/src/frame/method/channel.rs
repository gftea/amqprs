use amqp_serde::{types::{LongStr, ShortStr, ShortUint}, constants::REPLY_SUCCESS};
use serde::{Deserialize, Serialize};

use super::impl_mapping;
use crate::frame::{Frame, MethodHeader};

impl_mapping!(OpenChannel, 20, 10);
impl_mapping!(OpenChannelOk, 20, 11);
impl_mapping!(CloseChannel, 20, 40);
impl_mapping!(CloseChannelOk, 20, 41);

#[derive(Debug, Serialize)]
pub struct OpenChannel {
    out_of_band: ShortStr,
}
impl Default for OpenChannel {
    fn default() -> Self {
        Self {
            out_of_band: "".try_into().unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OpenChannelOk {
    channel_id: LongStr,
}
impl Default for OpenChannelOk {
    fn default() -> Self {
        Self {
            channel_id: "".try_into().unwrap(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CloseChannel {
    pub reply_code: ShortUint,
    pub reply_text: ShortStr,
    pub class_id: ShortUint,
    pub method_id: ShortUint,
}
impl Default for CloseChannel {
    fn default() -> Self {
        Self {
            reply_code: REPLY_SUCCESS,
            reply_text: "".try_into().unwrap(),
            class_id: 0,
            method_id: 0,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CloseChannelOk;
impl Default for CloseChannelOk {
    fn default() -> Self {
        Self {  }
    }
}