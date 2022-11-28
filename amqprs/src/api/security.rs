use amqp_serde::{
    to_buffer,
    types::{LongStr, ShortStr},
};
use bytes::BytesMut;

pub struct SecurityCredentials {
    pub username: String,
    pub password: String,
    mechanism: AuthenticationMechanism,
}

#[non_exhaustive]
enum AuthenticationMechanism {
    PLAIN,
    AMQPLAIN,
    // EXTERNAL,
    // RABBIT-CR-DEMO,
}

impl SecurityCredentials {
    pub fn new_plain(username: String, password: String) -> Self {
        Self {
            username,
            password,
            mechanism: AuthenticationMechanism::PLAIN,
        }
    }
    pub fn new_amqplain(username: String, password: String) -> Self {
        Self {
            username,
            password,
            mechanism: AuthenticationMechanism::AMQPLAIN,
        }
    }
    pub(crate) fn get_mechanism_name(&self) -> &str {
        match self.mechanism {
            AuthenticationMechanism::PLAIN => "PLAIN",
            AuthenticationMechanism::AMQPLAIN => "AMQPLAIN",
        }
    }
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
        }
    }
}
