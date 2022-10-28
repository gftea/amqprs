use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ContentBody {
    pub(crate) inner: Vec<u8>,
}

impl ContentBody {
    pub fn new(inner: Vec<u8>) -> Self {
        Self { inner }
    }
}
