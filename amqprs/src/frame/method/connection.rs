use std::fmt;

use crate::frame::REPLY_SUCCESS;
use amqp_serde::types::{AmqpPeerProperties, Bit, LongStr, LongUint, Octect, ShortStr, ShortUint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Start {
    pub version_major: Octect,
    pub version_minor: Octect,
    pub server_properties: AmqpPeerProperties,
    pub mechanisms: LongStr,
    pub locales: LongStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartOk {
    pub client_properties: AmqpPeerProperties,
    pub machanisms: ShortStr,
    pub response: LongStr,
    pub locale: ShortStr,
}

impl Default for StartOk {
    fn default() -> Self {
        Self {
            client_properties: AmqpPeerProperties::new(),
            machanisms: "PLAIN".try_into().unwrap(),
            response: "\0user\0bitnami".try_into().unwrap(),
            locale: "en_US".try_into().unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tune {
    pub channel_max: ShortUint,
    pub frame_max: LongUint,
    pub heartbeat: ShortUint,
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TuneOk {
    // RabbitMQ doesn't put a limit on channel-max, and treats any number in tune-ok as valid.
    // It does put a limit on frame-max, and checks that the value sent in tune-ok
    // is less than or equal.
    pub channel_max: ShortUint,
    pub frame_max: LongUint,
    pub heartbeat: ShortUint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Open {
    pub virtual_host: ShortStr,
    pub capabilities: ShortStr,
    pub insist: Bit,
}

impl Default for Open {
    fn default() -> Self {
        Self {
            virtual_host: "/".try_into().unwrap(),
            capabilities: ShortStr::default(),
            insist: false as Bit,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenOk {
    pub know_hosts: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Close {
    reply_code: ShortUint,
    reply_text: ShortStr,
    class_id: ShortUint,
    method_id: ShortUint,
}
impl fmt::Display for Close {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "Close connection due to '{}: {}', (class_id = {}, method_id = {})",
            self.reply_code(),
            self.reply_text(),
            self.class_id(),
            self.method_id()
        ))
    }
}
impl Close {
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
impl Default for Close {
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
pub struct CloseOk;

#[derive(Debug, Serialize, Deserialize)]
pub struct Secure {
    pub challenge: LongStr,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SecureOk {
    pub response: LongStr,
}

// below from https://www.rabbitmq.com/resources/specs/amqp0-9-1.extended.xml
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Blocked {
    pub reason: ShortStr,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Unblocked;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSecret {
    pub new_secret: LongStr,
    pub reason: ShortStr,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UpdateSecretOk;
