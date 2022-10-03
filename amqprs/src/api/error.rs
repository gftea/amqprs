use std::fmt;
use tokio::sync::mpsc::error::SendError;
use crate::net;

#[derive(Debug)]
pub enum Error {
    ConnectionOpenError,
    ChannelOpenError,
    ChannelUseError,
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
            Error::ConnectionOpenError => f.write_str("failed to open amqp connection"),
            Error::CommunicationError(msg) => write!(f, "{}", msg),
            Error::ChannelOpenError => f.write_str("failed to open amqp channel"),
            Error::ChannelUseError => f.write_str("error occurred in channel"),
        }
    }
}

impl std::error::Error for Error {}
