use std::fmt;

use crate::frame::REPLY_SUCCESS;
use amqp_serde::types::{Boolean, LongStr, ShortStr, ShortUint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OpenChannel {
    out_of_band: ShortStr,
}

impl OpenChannel {
    pub(crate) fn new() -> Self {
        Self {
            out_of_band: "".try_into().unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenChannelOk {
    pub(crate) channel_id: LongStr,
}

/// Used by channel [`close`] callback.
///
/// AMQP method frame [close](https://www.rabbitmq.com/amqp-0-9-1-reference.html#channel.close).
///
/// [`close`]: callbacks/trait.ChannelCallback.html#tymethod.close
// TX + RX
#[derive(Debug, Serialize, Deserialize)]
pub struct CloseChannel {
    reply_code: ShortUint,
    reply_text: ShortStr,
    class_id: ShortUint,
    method_id: ShortUint,
}

impl CloseChannel {
    pub(crate) fn new(
        reply_code: ShortUint,
        reply_text: ShortStr,
        class_id: ShortUint,
        method_id: ShortUint,
    ) -> Self {
        Self {
            reply_code,
            reply_text,
            class_id,
            method_id,
        }
    }

    pub fn reply_code(&self) -> u16 {
        self.reply_code
    }

    pub fn reply_text(&self) -> &String {
        self.reply_text.as_ref()
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
            "'{}: {}', (class_id = {}, method_id = {})",
            self.reply_code(),
            self.reply_text(),
            self.class_id(),
            self.method_id()
        ))
    }
}
impl Default for CloseChannel {
    // Compilance:
    // Indicates that the method completed successfully. This reply code is
    // reserved for future use - the current protocol design does not use positive
    // confirmation and reply codes are sent only in case of an error.
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

// TX + RX
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Flow {
    pub(crate) active: Boolean,
}

impl Flow {
    pub fn new(active: Boolean) -> Self {
        Self { active }
    }
}

// TX + RX
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FlowOk {
    pub(crate) active: Boolean,
}

impl FlowOk {
    pub fn new(active: Boolean) -> Self {
        Self { active }
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
