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
macro_rules! impl_mapping {
    ($name:ident, $class_id:literal, $method_id:literal) => {
        impl $name {
            pub fn header() -> &'static MethodHeader {                
                static __METHOD_HEADER: MethodHeader = 
                MethodHeader {
                    class_id: $class_id,
                    method_id: $method_id,
                };
                &__METHOD_HEADER
            }
            pub fn into_frame(self) -> Frame {                
                Frame::$name(
                    Self::header(),
                    self,
                )
            }
        }
    };
}
pub(super) use impl_mapping;