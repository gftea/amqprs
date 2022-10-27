use crate::net;
use amqp_serde::types::AmqpChannelId;
use std::fmt;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug)]
pub enum Error {
    ConnectionOpenError(String),
    ConnectionCloseError(String),
    ConnectionUseError(String),
    ChannelOpenError(String),
    ChannelCloseError(String),
    ChannelUseError(String),
    NetworkError(String),
    ChannelAlreadyClosed(String),
    ChannelAllocationError(String),
    InternalChannelError(String),
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NetworkError(msg) => write!(f, "AMQP network error: {}", msg),
            Error::ConnectionOpenError(msg) => write!(f, "AMQP connection open error: {msg}"),
            Error::ConnectionCloseError(msg)=> write!(f, "AMQP connection close error: {msg}"),
            Error::ConnectionUseError(msg) => write!(f, "AMQP connection error: {msg}"),
            Error::ChannelOpenError(msg) => write!(f, "AMQP channel open error: {msg}"),
            Error::ChannelUseError(msg) => write!(f, "AMQP channel close error: {msg}"),
            Error::ChannelCloseError(msg) => write!(f, "AMQP channel error: {msg}"),
            Error::ChannelAlreadyClosed(msg) => write!(f, "AMQP channel already closed: {msg}"),
            Error::ChannelAllocationError(msg) => write!(f, "AMQP channel resource allocation error: {msg}"),
            Error::InternalChannelError(msg) => write!(f, "Local internal channel error: {msg}"),
            
        }
    }
}

impl std::error::Error for Error {}
