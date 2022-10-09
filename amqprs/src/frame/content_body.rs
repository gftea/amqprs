use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContentBody {
    inner: Vec<u8>,
}

impl ContentBody {
    pub fn new(inner: Vec<u8>) -> Self {
        Self { inner }
    }
}
