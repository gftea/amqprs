use std::fmt;

#[derive(Debug)]
pub enum Error {
    Corrupted,
    SerdeError(String),
}

impl From<amqp_serde::Error> for Error {
    fn from(err: amqp_serde::Error) -> Self {
        Self::SerdeError(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Corrupted => f.write_str("corrupted frame"),
            Error::SerdeError(msg) => write!(f, "serde error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
