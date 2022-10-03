use std::{fmt, io};

use super::Message;

#[derive(Debug)]
pub enum Error {
    IOFailure(String),
    ReadFailure(String),
    WriteFailure(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IOFailure(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IOFailure(msg) => write!(f, "{}", msg),
            Error::ReadFailure(msg) => write!(f, "{}", msg),
            Error::WriteFailure(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}
