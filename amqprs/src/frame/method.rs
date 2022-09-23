use amqp_serde::types::{LongStr, LongUint, Octect, PeerProperties, ShortStr, ShortUint};
use serde::{Deserialize, Serialize};

pub trait Method {
    fn get_class_id(&self) -> ShortUint;
    fn get_method_id(&self) -> ShortUint;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodHeader {
    class_id: ShortUint,
    method_id: ShortUint,
}

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
            header: MethodHeader {
                class_id: 10,
                method_id: 11,
            },
            client_properties: PeerProperties::new(),
            machanisms: ShortStr::from("PLAIN"),
            response: LongStr::from("\0user\0bitnami"),
            locale: ShortStr::from("en_US"),
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

// Channel, class id = 20
