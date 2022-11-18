use std::fmt;

use crate::frame::REPLY_SUCCESS;
use amqp_serde::types::{Boolean, LongStr, ShortStr, ShortUint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OpenChannel {
    out_of_band: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenChannelOk {
    pub channel_id: LongStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloseChannel {
    reply_code: ShortUint,
    reply_text: ShortStr,
    class_id: ShortUint,
    method_id: ShortUint,
}

impl CloseChannel {
    pub fn reply_code(&self) -> u16 {
        self.reply_code
    }

    pub fn reply_text(&self) -> &String {
        &self.reply_text
    }

    pub fn class_id(&self) -> u16 {
        self.class_id
    }

    pub fn method_id(&self) -> u16 {
        self.method_id
    }
}

impl fmt::Display for CloseChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "Close channel due to '{}: {}', (class_id = {}, method_id = {})",
            self.reply_code(),
            self.reply_text(),
            self.class_id(),
            self.method_id()
        ))
    }
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
    active: Boolean,
}

impl Flow {
    pub fn new(active: Boolean) -> Self { Self { active } }

    pub fn active(&self) -> bool {
        self.active
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FlowOk {
    active: Boolean,
}

impl FlowOk {
    pub fn new(active: Boolean) -> Self { Self { active } }

    pub fn active(&self) -> bool {
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::Flow;

    #[test]
    fn test_default() {
        assert_eq!(false, Flow::default().active);
    }
}
