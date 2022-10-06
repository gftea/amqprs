use amqp_serde::types::ShortUint;
use serde::{Deserialize, Serialize};

//////////////////////////////////////////////////////////
mod basic;
mod channel;
mod connection;
mod exchange;
mod queue;
mod tx;
mod confirm;

pub use basic::*;
pub use channel::*;
pub use connection::*;
pub use exchange::*;
pub use queue::*;
pub use tx::*;
pub use confirm::*;
//////////////////////////////////////////////////////////
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MethodHeader {
    class_id: ShortUint,
    method_id: ShortUint,
}

impl MethodHeader {
    pub const fn new(class_id: ShortUint, method_id: ShortUint) -> Self {
        Self {
            class_id,
            method_id,
        }
    }

    pub fn class_id(&self) -> ShortUint {
        self.class_id
    }

    pub fn method_id(&self) -> ShortUint {
        self.method_id
    }
}
