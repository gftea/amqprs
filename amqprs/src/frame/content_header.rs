use std::fmt;

use amqp_serde::types::{FieldTable, LongLongUint, Octect, ShortStr, ShortUint, TimeStamp};
use serde::{de::Visitor, Deserialize, Serialize};

use crate::api::channel::ServerSpecificArguments;

use super::Frame;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentHeader {
    pub common: ContentHeaderCommon,
    pub basic_properties: BasicProperties,
}

impl ContentHeader {
    pub fn new(common: ContentHeaderCommon, basic_properties: BasicProperties) -> Self {
        Self {
            common,
            basic_properties,
        }
    }

    pub fn into_frame(self) -> Frame {
        Frame::ContentHeader(self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentHeaderCommon {
    pub class: ShortUint,
    pub weight: ShortUint,
    pub body_size: LongLongUint,
}



#[derive(Debug, Serialize, Default)]
pub struct BasicProperties {
    // // property flags is included in this type
    // // in order to manage the value according to optional property
    property_flags: [Octect; 2],

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

impl BasicProperties {
    pub fn new(
        content_type: Option<String>,
        content_encoding: Option<String>,
        headers: Option<ServerSpecificArguments>,
        delivery_mode: Option<u8>,
        priority: Option<u8>,
        correlation_id: Option<String>,
        reply_to: Option<String>,
        expiration: Option<String>,
        message_id: Option<String>,
        timestamp: Option<TimeStamp>,
        typ: Option<String>,
        user_id: Option<String>,
        app_id: Option<String>,
        cluster_id: Option<String>,
    ) -> Self {
        let mut property_flags = [0u8; 2];
        // first byte
        let content_type = match content_type {
            Some(v) => {
                property_flags[0] |= 1 << 7;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let content_encoding = match content_encoding {
            Some(v) => {
                property_flags[0] |= 1 << 6;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let headers = match headers {
            Some(v) => {
                property_flags[0] |= 1 << 5;
                Some(v.into_field_table())
            }
            None => None,
        };
        let delivery_mode = match delivery_mode {
            Some(v) => {
                property_flags[0] |= 1 << 4;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let correlation_id = match correlation_id {
            Some(v) => {
                property_flags[0] |= 1 << 2;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let reply_to = match reply_to {
            Some(v) => {
                property_flags[0] |= 1 << 1;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let expiration = match expiration {
            Some(v) => {
                property_flags[0] |= 1 << 0;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        // 2nd byte
        let message_id = match message_id {
            Some(v) => {
                property_flags[1] |= 1 << 7;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let typ = match typ {
            Some(v) => {
                property_flags[1] |= 1 << 5;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let user_id = match user_id {
            Some(v) => {
                property_flags[1] |= 1 << 4;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let app_id = match app_id {
            Some(v) => {
                property_flags[1] |= 1 << 3;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let cluster_id = match cluster_id {
            Some(v) => {
                property_flags[1] |= 1 << 2;
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        Self {
            property_flags,
            content_type,
            content_encoding,
            headers,
            delivery_mode,
            priority,
            correlation_id,
            reply_to,
            expiration,
            message_id,
            timestamp,
            typ,
            user_id,
            app_id,
            cluster_id,
        }
    }

    pub fn content_type(&self) -> Option<&String> {
        self.content_type.as_deref()
    }

    pub fn content_encoding(&self) -> Option<&String> {
        self.content_encoding.as_deref()
    }

    // pub fn headers(&self) -> Option<&HashMap<String, FieldValue>> {
    //     self.headers.as_deref()
    // }

    pub fn delivery_mode(&self) -> Option<u8> {
        self.delivery_mode
    }

    pub fn priority(&self) -> Option<u8> {
        self.priority
    }

    pub fn correlation_id(&self) -> Option<&String> {
        self.correlation_id.as_deref()
    }

    pub fn reply_to(&self) -> Option<&String> {
        self.reply_to.as_deref()
    }

    pub fn expiration(&self) -> Option<&String> {
        self.expiration.as_deref()
    }

    pub fn message_id(&self) -> Option<&String> {
        self.message_id.as_deref()
    }

    pub fn timestamp(&self) -> Option<u64> {
        self.timestamp
    }

    pub fn typ(&self) -> Option<&String> {
        self.typ.as_deref()
    }

    pub fn user_id(&self) -> Option<&String> {
        self.user_id.as_deref()
    }

    pub fn app_id(&self) -> Option<&String> {
        self.app_id.as_deref()
    }

    pub fn cluster_id(&self) -> Option<&String> {
        self.cluster_id.as_deref()
    }
}

impl<'de> Deserialize<'de> for BasicProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &[
            "content_type",
            "content_encoding",
            "headers",
            "delivery_mode",
            "priority",
            "correlation_id",
            "reply_to",
            "expiration",
            "message_id",
            "timestamp",
            "typ",
            "user_id",
            "app_id",
            "cluster_id",
        ];

        struct BasicPropertitiesVisitor;

        impl<'de> Visitor<'de> for BasicPropertitiesVisitor {
            type Value = BasicProperties;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct BasicPropertities")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let flags: [Octect; 2] = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let mut basic_properties = BasicProperties {
                    property_flags: flags,
                    content_type: None,
                    content_encoding: None,
                    headers: None,
                    delivery_mode: None,
                    priority: None,
                    correlation_id: None,
                    reply_to: None,
                    expiration: None,
                    message_id: None,
                    timestamp: None,
                    typ: None,
                    user_id: None,
                    app_id: None,
                    cluster_id: None,
                };
                if (flags[0] & 1 << 7) != 0 {
                    basic_properties.content_type = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[0] & 1 << 6) != 0 {
                    basic_properties.content_encoding = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[0] & 1 << 5) != 0 {
                    basic_properties.headers = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[0] & 1 << 4) != 0 {
                    basic_properties.delivery_mode = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[0] & 1 << 3) != 0 {
                    basic_properties.priority = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[0] & 1 << 2) != 0 {
                    basic_properties.correlation_id = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[0] & 1 << 1) != 0 {
                    basic_properties.reply_to = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[0] & 1 << 0) != 0 {
                    basic_properties.expiration = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                // 2nd byte of flags
                if (flags[1] & 1 << 7) != 0 {
                    basic_properties.message_id = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[1] & 1 << 6) != 0 {
                    basic_properties.timestamp = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[1] & 1 << 5) != 0 {
                    basic_properties.typ = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[1] & 1 << 4) != 0 {
                    basic_properties.user_id = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[1] & 1 << 3) != 0 {
                    basic_properties.app_id = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }
                if (flags[1] & 1 << 2) != 0 {
                    basic_properties.cluster_id = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                }

                Ok(basic_properties)
            }
        }
        deserializer.deserialize_struct("BasicPropertities", FIELDS, BasicPropertitiesVisitor)
    }
}
