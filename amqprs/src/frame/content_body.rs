use serde::Serialize;

use super::Frame;

#[derive(Debug, Serialize)]
pub struct ContentBody {
    pub(crate) inner: Vec<u8>,
}

impl ContentBody {
    pub fn new(inner: Vec<u8>) -> Self {
        Self { inner }
    }
    pub fn into_frame(self) -> Frame {
        Frame::ContentBody(self)
    }
}
