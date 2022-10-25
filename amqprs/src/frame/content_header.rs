use amqp_serde::types::{FieldTable, LongLongUint, Octect, ShortStr, ShortUint, TimeStamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentHeader {
    class: ShortUint,
    weight: ShortUint,
    body_size: LongLongUint,
    // TODO: customized deserialzer
    property_flags: (Octect, Octect),
}
// TODO: customized deserialzer

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicPropertities {
    // // property flags is included in this type
    // // in order to manage the value according to optional property
    // property_flags: ShortUint,
    content_type: Option<ShortStr>,
    content_encoding: Option<ShortStr>,
    headers: Option<FieldTable>,
    delivery_mode: Option<Octect>,
    priority: Option<Octect>,
    correlation_id: Option<ShortStr>,
    reply_to: Option<ShortStr>,
    expiration: Option<ShortStr>,
    message_id: Option<ShortStr>,
    timestamp: Option<TimeStamp>,
    typ: Option<ShortStr>,
    user_id: Option<ShortStr>,
    app_id: Option<ShortStr>,
    cluster_id: Option<ShortStr>,
}
