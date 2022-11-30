use std::fmt;

use amqp_serde::types::{FieldTable, FieldValue};
use serde::{Deserialize, Serialize};

use self::error::Error;
pub(in crate::api) type Result<T> = std::result::Result<T, Error>;

// macro should appear before module declaration
#[macro_use]
pub(crate) mod helpers {

    macro_rules! synchronous_request {
        ($tx:expr, $msg:expr, $rx:expr, $response:path, $err:path) => {{
            $tx.send($msg).await?;
            match $rx.await? {
                $response(_, method) => Ok(method),
                unexpected => Err($err(unexpected.to_string())),
            }
        }};
    }

    macro_rules! unwrap_expected_method {
        ($frame:expr, $variant:path, $err:expr) => {
            match $frame {
                $variant(_, method) => Ok(method),
                _ => Err($err),
            }
        };
    }

    // implements the API for chaining
    macro_rules! impl_builder_api {
        ($(#[$($attrss:tt)*])* $field_name:ident, $input_type:ty, $value:expr) => {
            $(#[$($attrss)*])*
            pub fn $field_name(&mut self, $field_name: $input_type) -> &mut Self {
                self.$field_name = $value;
                self
            }
            
        };
    }

    pub(crate) use impl_builder_api;
   
}



/////////////////////////////////////////////////////////////////////////////
pub mod callbacks;
pub mod channel;
pub mod connection;
pub mod consumer;
pub mod error;
pub mod security;
