use amqp_serde::types::{
    Bit, LongStr, LongUint, Octect, Path, PeerProperties, ShortStr, ShortUint,
};
use serde::{Deserialize, Serialize};

macro_rules! gen_method_header {
    ($name:ident, $class_id:literal, $method_id:literal) => {
        impl $name {
            fn get_header() -> MethodHeader {
                MethodHeader {
                    class_id: $class_id,
                    method_id: $method_id,
                }
            }
        }
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodHeader {
    class_id: ShortUint,
    method_id: ShortUint,
}

gen_method_header!(Start, 10, 10);
gen_method_header!(StartOk, 10, 11);
gen_method_header!(Tune, 10, 30);
gen_method_header!(TuneOk, 10, 31);
gen_method_header!(Open, 10, 40);
gen_method_header!(OpenOk, 10, 41);

/////////////////////////////////
/// Connection
/////////////////////////////////
#[derive(Debug, Deserialize)]
pub struct Start {
    header: MethodHeader,
    pub version_major: Octect,
    pub version_minor: Octect,
    pub server_properties: PeerProperties,
    pub mechanisms: LongStr,
    pub locales: LongStr,
}

#[derive(Debug, Serialize)]
pub struct StartOk {
    header: MethodHeader,
    pub client_properties: PeerProperties,
    pub machanisms: ShortStr,
    pub response: LongStr,
    pub locale: ShortStr,
}

impl Default for StartOk {
    fn default() -> Self {
        Self {
            header: Self::get_header(),
            client_properties: PeerProperties::new(),
            machanisms: "PLAIN".try_into().unwrap(),
            response: "\0user\0bitnami".try_into().unwrap(),
            locale: "en_US".try_into().unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Tune {
    header: MethodHeader,
    pub channel_max: ShortUint,
    pub frame_max: LongUint,
    pub heartbeat: ShortUint,
}
#[derive(Debug, Serialize)]
pub struct TuneOk {
    header: MethodHeader,
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
            header: Self::get_header(),
            channel_max: 0,
            frame_max: 0,
            heartbeat: 0,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Open {
    header: MethodHeader,
    pub virtual_host: ShortStr,
    pub capabilities: ShortStr,
    pub insist: Bit,
}

impl Default for Open {
    fn default() -> Self {
        Self {
            header: Self::get_header(),
            virtual_host: "/".try_into().unwrap(),
            capabilities: "".try_into().unwrap(),
            insist: false as Bit,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct OpenOk {
    header: MethodHeader,
    pub know_hosts: ShortStr,
}
//////////////////////////////////////////////////////////////
// Channel, class id = 20
