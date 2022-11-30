use amqp_serde::types::{FieldTable, FieldValue};

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
/// A user-facing type act as adaption layer for AMQP's Field Table type.
/// The semantics of these arguments depends on the server implementation or user application.
#[derive(Debug, Clone, Default)]
pub struct AmqArgumentTable {
    table: FieldTable,
}

impl AmqArgumentTable {
    pub fn new() -> Self {
        Self {
            table: FieldTable::new(),
        }
    }

    pub fn insert_string(&mut self, key: String, value: String) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::S(value.try_into().unwrap()),
        );
    }
    pub fn insert_bool(&mut self, key: String, value: bool) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::t(value.try_into().unwrap()),
        );
    }
    pub fn insert_u8(&mut self, key: String, value: u8) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::B(value.try_into().unwrap()),
        );
    }
    pub fn insert_u16(&mut self, key: String, value: u8) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::u(value.try_into().unwrap()),
        );
    }
    pub fn insert_u32(&mut self, key: String, value: u32) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::i(value.try_into().unwrap()),
        );
    }
    pub fn insert_i64(&mut self, key: String, value: i64) {
        self.table.insert(
            key.try_into().unwrap(),
            FieldValue::l(value.try_into().unwrap()),
        );
    }
    /// the field table type should be hidden from API
    pub(crate) fn into_field_table(self) -> FieldTable {
        self.table
    }
}

/////////////////////////////////////////////////////////////////////////////
pub mod callbacks;
pub mod channel;
pub mod connection;
pub mod consumer;
pub mod error;
pub mod security;
