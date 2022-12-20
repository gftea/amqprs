use std::{fmt, io};

use crate::frame;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug)]
pub(crate) enum Error {
    NetworkIo(String),
    SyncChannel(String),
    Serde(String),
    Framing(String),
    Callback,
    PeerShutdown,
    Interrupted,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::NetworkIo(err.to_string())
    }
}
impl From<amqp_serde::Error> for Error {
    fn from(err: amqp_serde::Error) -> Self {
        Error::Serde(err.to_string())
    }
}
impl From<frame::Error> for Error {
    fn from(err: frame::Error) -> Self {
        Error::Framing(err.to_string())
    }
}
impl<T> From<SendError<T>> for Error {
    fn from(err: SendError<T>) -> Self {
        Error::SyncChannel(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NetworkIo(msg) => write!(f, "network io error: {}", msg),
            Error::SyncChannel(msg) => write!(f, "internal communication error: {}", msg),
            Error::Serde(msg) => write!(f, "serde error: {}", msg),
            Error::Framing(msg) => write!(f, "framing error: {}", msg),
            Error::Callback => write!(f, "callback error"),
            Error::PeerShutdown => f.write_str("peer shutdown"),
            Error::Interrupted => f.write_str("connection interrupted"),
        }
    }
}

impl std::error::Error for Error {}
