use super::impl_mapping;
use crate::frame::{Frame, MethodHeader, REPLY_SUCCESS};
use amqp_serde::types::{AmqpPeerProperties, Bit, LongStr, LongUint, Octect, ShortStr, ShortUint};
use serde::{Deserialize, Serialize};

impl_mapping!(Start, 10, 10);
impl_mapping!(StartOk, 10, 11);
impl_mapping!(Tune, 10, 30);
impl_mapping!(TuneOk, 10, 31);
impl_mapping!(Open, 10, 40);
impl_mapping!(OpenOk, 10, 41);
impl_mapping!(Close, 10, 50);
impl_mapping!(CloseOk, 10, 51);

/////////////////////////////////
/// Connection
/////////////////////////////////
#[derive(Debug, Deserialize)]
pub struct Start {
    pub version_major: Octect,
    pub version_minor: Octect,
    pub server_properties: AmqpPeerProperties,
    pub mechanisms: LongStr,
    pub locales: LongStr,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
pub struct Tune {
    pub channel_max: ShortUint,
    pub frame_max: LongUint,
    pub heartbeat: ShortUint,
}
#[derive(Debug, Serialize)]
pub struct TuneOk {
    pub channel_max: ShortUint,
    pub frame_max: LongUint,
    pub heartbeat: ShortUint,
}
// RabbitMQ doesn't put a limit on channel-max, and treats any number in tune-ok as valid.
// It does put a limit on frame-max, and checks that the value sent in tune-ok
// is less than or equal.

impl Default for TuneOk {
    fn default() -> Self {
        Self {
            channel_max: 0,
            frame_max: 0,
            heartbeat: 0,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Open {
    pub virtual_host: ShortStr,
    pub capabilities: ShortStr,
    pub insist: Bit,
}

impl Default for Open {
    fn default() -> Self {
        Self {
            virtual_host: "/".try_into().unwrap(),
            capabilities: "".try_into().unwrap(),
            insist: false as Bit,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct OpenOk {
    pub know_hosts: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Close {
    pub reply_code: ShortUint,
    pub reply_text: ShortStr,
    pub class_id: ShortUint,
    pub method_id: ShortUint,
}
impl Default for Close {
    fn default() -> Self {
        Self {
            reply_code: REPLY_SUCCESS,
            reply_text: "".try_into().unwrap(),
            class_id: 0,
            method_id: 0,
        }
    }
}
impl Close {
    pub fn new(
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloseOk;
impl Default for CloseOk {
    fn default() -> Self {
        Self
    }
}
