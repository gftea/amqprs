use amqp_serde::types::{Octect, PeerProperties, LongStr, ShortStr, ShortUint, LongUint};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodPayload<T> {
    pub class_id: ShortUint, 
    pub method_id: ShortUint,
    pub method: T,
}

// Connection, class id = 10
#[derive(Debug, Serialize, Deserialize)]
pub struct Start {
   pub version_major: Octect,
   pub version_minor: Octect,
   pub server_properties: PeerProperties,
   pub mechanisms: LongStr,
   pub locales: LongStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartOk {
   pub client_properties: PeerProperties,
   pub machanisms: ShortStr,
   pub response: LongStr,
   pub locale: ShortStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tune {
   channel_max: ShortUint,
   frame_max: LongUint,
   heartbeat: ShortUint,
}

// Channel, class id = 20
