use serde::{Deserialize, Serialize};

// deprecated: https://www.rabbitmq.com/spec-differences.html
#[derive(Debug, Serialize, Deserialize)]
pub struct Request;

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestOk;
