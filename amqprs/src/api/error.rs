use std::fmt;

#[derive(Debug)]
pub enum Error {

    Other(String),

}

impl From<amqp_serde::Error> for Error {
    fn from(err: amqp_serde::Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Error::Incomplete => f.write_str("incomplete frame"),
            // Error::Corrupted => f.write_str("corrupted frame"),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}