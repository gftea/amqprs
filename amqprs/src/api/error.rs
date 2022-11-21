use crate::net;

use std::fmt;
use tokio::sync::{mpsc::error::SendError, oneshot::error::RecvError};

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    ConnectionOpenError(String),
    ConnectionCloseError(String),
    ConnectionUseError(String),
    ChannelOpenError(String),
    ChannelCloseError(String),
    ChannelUseError(String),
    NetworkError(String),
    InternalChannelError(String),    
    Other(String),
}

impl From<net::Error> for Error {
    fn from(err: net::Error) -> Self {
        Self::NetworkError(err.to_string())
    }
}
impl<T> From<SendError<T>> for Error {
    fn from(err: SendError<T>) -> Self {
        Self::InternalChannelError(err.to_string())
    }
}
impl From<RecvError> for Error {
    fn from(err: RecvError) -> Self {
        Self::InternalChannelError(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NetworkError(msg) => write!(f, "AMQP network error: {}", msg),
            Error::ConnectionOpenError(msg) => write!(f, "AMQP connection open error: {msg}"),
            Error::ConnectionCloseError(msg) => write!(f, "AMQP connection close error: {msg}"),
            Error::ConnectionUseError(msg) => write!(f, "AMQP connection usage error: {msg}"),
            Error::ChannelOpenError(msg) => write!(f, "AMQP channel open error: {msg}"),
            Error::ChannelUseError(msg) => write!(f, "AMQP channel close error: {msg}"),
            Error::ChannelCloseError(msg) => write!(f, "AMQP channel usage error: {msg}"),
            Error::InternalChannelError(msg) => {
                write!(f, "internal communication error: {msg}")
            }
            Error::Other(msg) => write!(f, "other error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
