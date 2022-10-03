use std::{fmt, io};

use tokio::sync::mpsc::error::SendError;

use crate::net;

#[derive(Debug)]
pub enum Error {
    ConnectionOpenFailure,
    ChannelOpenFailure,
    ChannelUseFailure,
    NetworkFailure(String),
    Other(String),
}

impl From<net::Error> for Error {
    fn from(err: net::Error) -> Self {
        Self::NetworkFailure(err.to_string())
    }
}
impl From<amqp_serde::Error> for Error {
    fn from(err: amqp_serde::Error) -> Self {
        Self::Other(err.to_string())
    }
}
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::NetworkFailure(err.to_string())
    }
}
impl<T> From<SendError<T>> for Error {
    fn from(err: SendError<T>) -> Self {
        Self::NetworkFailure(err.to_string())
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ConnectionOpenFailure => f.write_str("open connection failure"),
            Error::NetworkFailure(msg) => write!(f, "{}", msg),
            Error::ChannelOpenFailure => f.write_str("open channel failure"),
            Error::ChannelUseFailure => f.write_str("use channel failure"),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}
