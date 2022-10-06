use std::fmt;
use amqp_serde::types::AmqpChannelId;
use tokio::sync::mpsc::error::SendError;
use crate::net;

#[derive(Debug)]
pub enum Error {
    ConnectionOpenError(String),
    ConnectionCloseError(String),
    ConnectionUseError(String),
    ChannelOpenError(String),
    ChannelCloseError(String),
    ChannelUseError(String),
    CommunicationError(String),
}

impl From<net::Error> for Error {
    fn from(err: net::Error) -> Self {
        Self::CommunicationError(err.to_string())
    }
}
impl<T> From<SendError<T>> for Error {
    fn from(err: SendError<T>) -> Self {
        Self::CommunicationError(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CommunicationError(msg) => write!(f, "{}", msg),

            Error::ConnectionOpenError(msg) => write!(f, "AMQP connection open error: {msg}"),
            Error::ConnectionCloseError(msg)=> write!(f, "AMQP connection close error: {msg}"),
            Error::ConnectionUseError(msg) => write!(f, "AMQP connection error: {msg}"),
            Error::ChannelOpenError(msg) => write!(f, "AMQP channel open error: {msg}"),
            Error::ChannelUseError(msg) => write!(f, "AMQP channel close error: {msg}"),
            Error::ChannelCloseError(msg) => write!(f, "AMQP channel error: {msg}"),
            
        }
    }
}

impl std::error::Error for Error {}
