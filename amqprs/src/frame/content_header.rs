use std::fmt;

use amqp_serde::types::{FieldTable, LongLongUint, Octect, ShortStr, ShortUint, TimeStamp};
use serde::{de::Visitor, Deserialize, Serialize};

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
        Frame::ContentHeader(Box::new(self))
    }
}
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentHeaderCommon {
    pub class: ShortUint,
    pub weight: ShortUint,
    pub body_size: LongLongUint,
}

////////////////////////////////////////////////////////////////////////////////
/// AMQP message properties.
///
/// User is recommended to use the chainable setter to create desired propertities.
///
/// See also [message properties](https://www.rabbitmq.com/consumers.html#message-properties).
///
/// # Example
///
/// ```
/// # use amqprs::{BasicProperties, DELIVERY_MODE_PERSISTENT};
/// let basic_props = BasicProperties::default()
///     .with_content_type("application/json")
///     .with_delivery_mode(DELIVERY_MODE_PERSISTENT)
///     .with_user_id("user")
///     .with_app_id("consumer_test")
///     .finish();
/// ```
#[derive(Debug, Serialize, Default, Clone)]
pub struct BasicProperties {
    // property flags bits are included in order to
    // manage the value according to optional property
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
    message_type: Option<ShortStr>,
    user_id: Option<ShortStr>,
    app_id: Option<ShortStr>,
    cluster_id: Option<ShortStr>,
}

impl BasicProperties {
    #[allow(clippy::too_many_arguments)]
    /// Returns a new instance.
    pub fn new(
        content_type: Option<String>,
        content_encoding: Option<String>,
        headers: Option<FieldTable>,
        delivery_mode: Option<u8>,
        priority: Option<u8>,
        correlation_id: Option<String>,
        reply_to: Option<String>,
        expiration: Option<String>,
        message_id: Option<String>,
        timestamp: Option<TimeStamp>,
        message_type: Option<String>,
        user_id: Option<String>,
        app_id: Option<String>,
        cluster_id: Option<String>,
    ) -> Self {
        // initial flags
        let mut property_flags = [0u8; 2];

        // first byte of flags
        let content_type = match content_type {
            Some(v) => {
                Self::set_content_type_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let content_encoding = match content_encoding {
            Some(v) => {
                Self::set_content_encoding_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let headers = match headers {
            Some(v) => {
                Self::set_headers_flag(&mut property_flags);
                Some(v)
            }
            None => None,
        };
        let delivery_mode = match delivery_mode {
            Some(v) => {
                Self::set_delivery_mode_flag(&mut property_flags);
                Some(v)
            }
            None => None,
        };
        let priority = match priority {
            Some(v) => {
                Self::set_priority_flag(&mut property_flags);
                Some(v)
            }
            None => None,
        };
        let correlation_id = match correlation_id {
            Some(v) => {
                Self::set_correlation_id_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let reply_to = match reply_to {
            Some(v) => {
                Self::set_reply_to_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let expiration = match expiration {
            Some(v) => {
                Self::set_expiration_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        // second byte of flags
        let message_id = match message_id {
            Some(v) => {
                Self::set_message_id_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let timestamp = match timestamp {
            Some(v) => {
                Self::set_timestamp_flag(&mut property_flags);
                Some(v)
            }
            None => None,
        };
        let message_type = match message_type {
            Some(v) => {
                Self::set_message_type_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let user_id = match user_id {
            Some(v) => {
                Self::set_user_id_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let app_id = match app_id {
            Some(v) => {
                Self::set_app_id_flag(&mut property_flags);
                Some(v.try_into().unwrap())
            }
            None => None,
        };
        let cluster_id = match cluster_id {
            Some(v) => {
                Self::set_cluster_id_flag(&mut property_flags);
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
            message_type,
            user_id,
            app_id,
            cluster_id,
        }
    }

    fn set_content_type_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 7;
    }
    fn set_content_encoding_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 6;
    }
    fn set_headers_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 5;
    }
    fn set_delivery_mode_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 4;
    }
    fn set_priority_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 3;
    }
    fn set_correlation_id_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 2;
    }
    fn set_reply_to_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 1;
    }
    fn set_expiration_flag(flags: &mut [Octect; 2]) {
        flags[0] |= 1 << 0;
    }
    fn set_message_id_flag(flags: &mut [Octect; 2]) {
        flags[1] |= 1 << 7;
    }
    fn set_timestamp_flag(flags: &mut [Octect; 2]) {
        flags[1] |= 1 << 6;
    }
    fn set_message_type_flag(flags: &mut [Octect; 2]) {
        flags[1] |= 1 << 5;
    }
    fn set_user_id_flag(flags: &mut [Octect; 2]) {
        flags[1] |= 1 << 4;
    }
    fn set_app_id_flag(flags: &mut [Octect; 2]) {
        flags[1] |= 1 << 3;
    }
    fn set_cluster_id_flag(flags: &mut [Octect; 2]) {
        flags[1] |= 1 << 2;
    }

    pub fn content_type(&self) -> Option<&String> {
        self.content_type.as_ref().map(|v| v.as_ref())
    }

    /// Chainable setter of content type.
    ///
    /// # Default: [`None`]
    pub fn with_content_type(&mut self, content_type: &str) -> &mut Self {
        Self::set_content_type_flag(&mut self.property_flags);
        self.content_type = Some(content_type.to_owned().try_into().unwrap());
        self
    }

    pub fn content_encoding(&self) -> Option<&String> {
        self.content_encoding.as_ref().map(|v| v.as_ref())
    }

    /// Chainable setter of content encoding.
    ///
    /// # Default: [`None`]
    pub fn with_content_encoding(&mut self, content_encoding: &str) -> &mut Self {
        Self::set_content_encoding_flag(&mut self.property_flags);
        self.content_encoding = Some(content_encoding.to_owned().try_into().unwrap());
        self
    }

    pub fn headers(&self) -> Option<&FieldTable> {
        self.headers.as_ref()
    }

    /// Chainable setter of headers.
    ///
    /// # Default: [`None`]
    pub fn with_headers(&mut self, headers: FieldTable) -> &mut Self {
        Self::set_headers_flag(&mut self.property_flags);
        self.headers = Some(headers);
        self
    }

    pub fn delivery_mode(&self) -> Option<u8> {
        self.delivery_mode
    }

    /// Chainable setter of delivery mode.
    ///
    /// `delivery_mode`: either [`DELIVERY_MODE_TRANSIENT`] or [`DELIVERY_MODE_PERSISTENT`]
    ///
    /// # Default: [`None`]
    ///
    /// [`DELIVERY_MODE_TRANSIENT`]: ../constant.DELIVERY_MODE_TRANSIENT.html
    /// [`DELIVERY_MODE_PERSISTENT`]: ../constant.DELIVERY_MODE_PERSISTENT.html
    pub fn with_delivery_mode(&mut self, delivery_mode: u8) -> &mut Self {
        Self::set_delivery_mode_flag(&mut self.property_flags);
        self.delivery_mode = Some(delivery_mode);
        self
    }

    pub fn priority(&self) -> Option<u8> {
        self.priority
    }

    /// Chainable setter of priority.
    ///
    /// `priority`: message priority, 0 to 9
    ///
    /// # Default: [`None`]
    pub fn with_priority(&mut self, priority: u8) -> &mut Self {
        Self::set_priority_flag(&mut self.property_flags);
        self.priority = Some(priority);
        self
    }

    pub fn correlation_id(&self) -> Option<&String> {
        self.correlation_id.as_ref().map(|v| v.as_ref())
    }

    /// Chainable setter of correlation id.
    ///
    /// `correlation_id`: application correlation identifier
    ///
    /// # Default: [`None`]
    pub fn with_correlation_id(&mut self, correlation_id: &str) -> &mut Self {
        Self::set_correlation_id_flag(&mut self.property_flags);
        self.correlation_id = Some(correlation_id.try_into().unwrap());
        self
    }

    pub fn reply_to(&self) -> Option<&String> {
        self.reply_to.as_ref().map(|v| v.as_ref())
    }

    /// Chainable setter of reply_to.
    ///
    /// # Default: [`None`]
    pub fn with_reply_to(&mut self, reply_to: &str) -> &mut Self {
        Self::set_reply_to_flag(&mut self.property_flags);
        self.reply_to = Some(reply_to.try_into().unwrap());
        self
    }

    pub fn expiration(&self) -> Option<&String> {
        self.expiration.as_ref().map(|v| v.as_ref())
    }

    /// Chainable setter of expiration.
    ///
    /// # Default: [`None`]
    pub fn with_expiration(&mut self, expiration: &str) -> &mut Self {
        Self::set_expiration_flag(&mut self.property_flags);
        self.expiration = Some(expiration.try_into().unwrap());
        self
    }

    pub fn message_id(&self) -> Option<&String> {
        self.message_id.as_ref().map(|v| v.as_ref())
    }
    /// Chainable setter of message_id.
    ///
    /// # Default: [`None`]
    pub fn with_message_id(&mut self, message_id: &str) -> &mut Self {
        Self::set_message_id_flag(&mut self.property_flags);
        self.message_id = Some(message_id.try_into().unwrap());
        self
    }

    pub fn timestamp(&self) -> Option<u64> {
        self.timestamp
    }
    /// Chainable setter of timestamp.
    ///
    /// # Default: [`None`]
    pub fn with_timestamp(&mut self, timestamp: u64) -> &mut Self {
        Self::set_timestamp_flag(&mut self.property_flags);
        self.timestamp = Some(timestamp);
        self
    }

    pub fn message_type(&self) -> Option<&String> {
        self.message_type.as_ref().map(|v| v.as_ref())
    }
    /// Chainable setter of message_type.
    ///
    /// # Default: [`None`]
    pub fn with_message_type(&mut self, message_type: &str) -> &mut Self {
        Self::set_message_type_flag(&mut self.property_flags);
        self.message_type = Some(message_type.try_into().unwrap());
        self
    }
    pub fn user_id(&self) -> Option<&String> {
        self.user_id.as_ref().map(|v| v.as_ref())
    }
    /// Chainable setter of user_id.
    ///
    /// # Default: [`None`]
    pub fn with_user_id(&mut self, user_id: &str) -> &mut Self {
        Self::set_user_id_flag(&mut self.property_flags);
        self.user_id = Some(user_id.try_into().unwrap());
        self
    }

    pub fn app_id(&self) -> Option<&String> {
        self.app_id.as_ref().map(|v| v.as_ref())
    }
    /// Chainable setter of app_id.
    ///
    /// # Default: [`None`]
    pub fn with_app_id(&mut self, app_id: &str) -> &mut Self {
        Self::set_app_id_flag(&mut self.property_flags);
        self.app_id = Some(app_id.try_into().unwrap());
        self
    }

    pub fn cluster_id(&self) -> Option<&String> {
        self.cluster_id.as_ref().map(|v| v.as_ref())
    }
    /// Chainable setter of cluster_id.
    ///
    /// # Default: [`None`]
    pub fn with_cluster_id(&mut self, cluster_id: &str) -> &mut Self {
        Self::set_cluster_id_flag(&mut self.property_flags);
        self.cluster_id = Some(cluster_id.try_into().unwrap());
        self
    }

    /// Finish chaining and returns a new instance according to chained configurations.
    pub fn finish(&mut self) -> Self {
        self.clone()
    }
}

impl<'de> Deserialize<'de> for BasicProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
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
                    message_type: None,
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
                    basic_properties.message_type = seq
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

impl fmt::Display for BasicProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Basic Propertities: {{ ")?;
        if let Some(ref v) = self.content_type {
            write!(f, "content_type = {}, ", v)?;
        }
        if let Some(ref v) = self.content_encoding {
            write!(f, "content_encoding = {}, ", v)?;
        }
        if let Some(ref v) = self.headers {
            write!(f, "headers = {}, ", v)?;
        }
        if let Some(ref v) = self.delivery_mode {
            write!(f, "delivery_mode = {}, ", v)?;
        }
        if let Some(ref v) = self.priority {
            write!(f, "priority = {}, ", v)?;
        }
        if let Some(ref v) = self.correlation_id {
            write!(f, "correlation_id = {}, ", v)?;
        }
        if let Some(ref v) = self.reply_to {
            write!(f, "reply_to = {}, ", v)?;
        }
        if let Some(ref v) = self.expiration {
            write!(f, "expiration = {}, ", v)?;
        }
        if let Some(ref v) = self.message_id {
            write!(f, "message_id = {}, ", v)?;
        }

        if let Some(ref v) = self.timestamp {
            write!(f, "timestamp = {}, ", v)?;
        }
        if let Some(ref v) = self.message_type {
            write!(f, "message_type = {}, ", v)?;
        }
        if let Some(ref v) = self.user_id {
            write!(f, "user_id = {}, ", v)?;
        }
        if let Some(ref v) = self.app_id {
            write!(f, "app_id = {}, ", v)?;
        }
        if let Some(ref v) = self.cluster_id {
            write!(f, "cluster_id = {} ", v)?;
        }

        write!(f, "}}")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use amqp_serde::types::FieldTable;

    use crate::{BasicProperties, DELIVERY_MODE_TRANSIENT};

    #[test]
    fn test_basic_properties_internal_flags() {
        let props = BasicProperties::new(
            Some("application/text".to_owned()),
            Some("utf8".to_owned()),
            Some(FieldTable::new()),
            Some(DELIVERY_MODE_TRANSIENT),
            Some(1),
            Some("beef".to_owned()),
            Some("callback_queue".to_owned()),
            Some("Sun Jan 22 17:19:26 CET 2023".to_owned()),
            Some("101".to_owned()),
            Some(1674404425),
            Some("Ping".to_owned()),
            Some("user".to_owned()),
            Some("app".to_owned()),
            Some("my_cluster".to_owned()),
        );

        assert_eq!([0xff, 0xfc], props.property_flags);

        let mut props = BasicProperties::new(
            Some("application/text".to_owned()),
            None,
            None,
            None,
            Some(1),
            None,
            None,
            None,
            Some("101".to_owned()),
            None,
            None,
            None,
            Some("app".to_owned()),
            None,
        );
        assert_eq!([0x88, 0x88], props.property_flags);

        props.with_content_encoding("utf8");
        assert_eq!([0xC8, 0x88], props.property_flags);

        props.with_timestamp(1674404425);
        assert_eq!([0xC8, 0xC8], props.property_flags);
    }
}
