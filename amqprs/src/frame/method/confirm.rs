use amqp_serde::types::Boolean;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Select {
    no_wait: Boolean,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectOk;
