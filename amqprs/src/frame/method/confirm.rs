use amqp_serde::types::Boolean;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Select {
    no_wait: Boolean,
}

impl Select {
    pub fn new(no_wait: Boolean) -> Self {
        Self { no_wait }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectOk;
