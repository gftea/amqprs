use amqp_serde::types::ShortUint;
use serde::{Deserialize, Serialize};
//////////////////////////////////////////////////////////
mod basic;
mod channel;
mod connection;
mod exchange;
mod queue;
mod tx;
pub use basic::*;
pub use channel::*;
pub use connection::*;
pub use exchange::*;
pub use queue::*;
pub use tx::*;

//////////////////////////////////////////////////////////
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MethodHeader {
    class_id: ShortUint,
    method_id: ShortUint,
}

