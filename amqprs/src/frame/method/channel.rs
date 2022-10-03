use amqp_serde::types::{Boolean, LongStr, ShortStr, ShortUint};
use serde::{Deserialize, Serialize};

use super::impl_mapping;
use crate::frame::{Frame, MethodHeader, REPLY_SUCCESS};

impl_mapping!(OpenChannel, 20, 10);
impl_mapping!(OpenChannelOk, 20, 11);
impl_mapping!(Flow, 20, 20);
impl_mapping!(FlowOk, 20, 21);
impl_mapping!(CloseChannel, 20, 40);
impl_mapping!(CloseChannelOk, 20, 41);

#[derive(Debug, Serialize, Default)]
pub struct OpenChannel {
    out_of_band: ShortStr,
}

#[derive(Debug, Deserialize)]
pub struct OpenChannelOk {
    pub channel_id: LongStr,
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
            reply_text: ShortStr::default(),
            class_id: 0,
            method_id: 0,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CloseChannelOk;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Flow {
    pub active: Boolean,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FlowOk {
    pub active: Boolean,
}

#[cfg(test)]
mod tests {
    use super::Flow;

    #[test]
    fn test_default() {
        assert_eq!(0, Flow::default().active);
    }
}
