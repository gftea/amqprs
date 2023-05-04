//! Error type can be returned by the APIs.

use crate::net;

use std::fmt;
use tokio::sync::{mpsc::error::SendError, oneshot::error::RecvError};
#[cfg(feature = "urispec")]
use uriparse::uri_reference::URIReferenceError;

/// A list of errors can be returned by the APIs.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error when using an amqp(s) uri. Usually due to incorrect usage by user.
    UriError(String),
    /// Error during openning a connection.
    ConnectionOpenError(String),
    /// Error during closing a connection.
    ConnectionCloseError(String),
    /// Error when using the connection. Usually due to incorrect usage by user.
    ConnectionUseError(String),
    /// Error during openning a channel.
    ChannelOpenError(String),
    /// Error during closing a channel.
    ChannelCloseError(String),
    /// Error when using the channel. Usually due to incorrect usage by user.
    ChannelUseError(String),
    /// Error occurs in network layer.
    NetworkError(String),
    /// Error in sending or receiving messages via internal communication channel.
    /// Usually due to incorrect usage by user.
    InternalChannelError(String),
}

#[cfg(feature = "urispec")]
impl From<URIReferenceError> for Error {
    fn from(err: URIReferenceError) -> Self {
        Self::UriError(err.to_string())
    }
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
            Error::UriError(msg) => {
                write!(f, "AMQP(S) URI error: {}", msg)
            }
            Error::NetworkError(msg) => write!(f, "AMQP network error: {}", msg),
            Error::ConnectionOpenError(msg) => write!(f, "AMQP connection open error: {}", msg),
            Error::ConnectionCloseError(msg) => write!(f, "AMQP connection close error: {}", msg),
            Error::ConnectionUseError(msg) => write!(f, "AMQP connection usage error: {}", msg),
            Error::ChannelOpenError(msg) => write!(f, "AMQP channel open error: {}", msg),
            Error::ChannelUseError(msg) => write!(f, "AMQP channel usage error: {}", msg),
            Error::ChannelCloseError(msg) => write!(f, "AMQP channel close error: {}", msg),
            Error::InternalChannelError(msg) => {
                write!(f, "AMQP internal communication error: {}", msg)
            }
        }
    }
}

impl std::error::Error for Error {}
