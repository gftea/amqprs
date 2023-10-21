//! This module provides configuration API of Security and Access Control.
//!
//! The configuration is used as part of [`OpenConnectionArguments`] value.
//!
//! [`OpenConnectionArguments`]: ../connection/struct.OpenConnectionArguments.html
//! [`Connection::open`]: ../connection/struct.Connection.html#method.open
use amqp_serde::{
    to_buffer,
    types::{LongStr, ShortStr},
};
use bytes::BytesMut;

/// Credentials used to open a connection.
#[derive(Clone)]
pub struct SecurityCredentials {
    username: String,
    password: String,
    mechanism: AuthenticationMechanism,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
#[non_exhaustive]
enum AuthenticationMechanism {
    PLAIN,
    AMQPLAIN,
    EXTERNAL,
    // RABBIT-CR-DEMO,
}

impl SecurityCredentials {
    /// Create and return a SASL/PLAIN credential with given `username` and `password`.
    ///
    /// See [RabbitMQ access control](https://www.rabbitmq.com/access-control.html#mechanisms).
    pub fn new_plain(username: &str, password: &str) -> Self {
        Self {
            username: username.to_owned(),
            password: password.to_owned(),
            mechanism: AuthenticationMechanism::PLAIN,
        }
    }
    /// Create and return a AMQPLAIN credential with given `username` and `password`.
    ///
    /// See [RabbitMQ access control](https://www.rabbitmq.com/access-control.html#mechanisms).
    pub fn new_amqplain(username: &str, password: &str) -> Self {
        Self {
            username: username.to_owned(),
            password: password.to_owned(),
            mechanism: AuthenticationMechanism::AMQPLAIN,
        }
    }

    /// Create and return EXTERNAL without credentials
    ///
    /// This must be used together with mTLS connection.
    pub fn new_external() -> Self {
        Self {
            username: "".to_owned(),
            password: "".to_owned(),
            mechanism: AuthenticationMechanism::EXTERNAL,
        }
    }

    /// Get the name of authentication mechanism of current credential
    pub(crate) fn get_mechanism_name(&self) -> &str {
        match self.mechanism {
            AuthenticationMechanism::PLAIN => "PLAIN",
            AuthenticationMechanism::AMQPLAIN => "AMQPLAIN",
            AuthenticationMechanism::EXTERNAL => "EXTERNAL",
        }
    }
    /// Get the security challenge `response` string, to be sent to server.
    pub(crate) fn get_response(&self) -> String {
        match self.mechanism {
            AuthenticationMechanism::PLAIN => format!("\0{}\0{}", self.username, self.password),
            AuthenticationMechanism::AMQPLAIN => {
                let mut buf = BytesMut::new();
                to_buffer(
                    &<&str as TryInto<ShortStr>>::try_into("LOGIN").unwrap(),
                    &mut buf,
                )
                .unwrap();
                to_buffer(&'S', &mut buf).unwrap();
                to_buffer(
                    &<&str as TryInto<LongStr>>::try_into(&self.username).unwrap(),
                    &mut buf,
                )
                .unwrap();

                to_buffer(
                    &<&str as TryInto<ShortStr>>::try_into("PASSWORD").unwrap(),
                    &mut buf,
                )
                .unwrap();
                to_buffer(&'S', &mut buf).unwrap();
                to_buffer(
                    &<&str as TryInto<LongStr>>::try_into(&self.password).unwrap(),
                    &mut buf,
                )
                .unwrap();
                String::from_utf8(buf.to_vec()).unwrap()
            }
            AuthenticationMechanism::EXTERNAL => "".to_string(),
        }
    }
}
