pub use amqp_serde::types::FieldTable;

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

    macro_rules! impl_chainable_setter {
        ($(#[$($attrss:tt)*])* $field_name:ident, $input_type:ty) => {
            $(#[$($attrss)*])*
            pub fn $field_name(&mut self, $field_name: $input_type) -> &mut Self {
                self.$field_name = $field_name;
                self
            }

        };
    }

    macro_rules! impl_chainable_alias_setter {
        ($(#[$($attrss:tt)*])* $method_name:ident, $field_name:ident, $input_type:ty) => {
            $(#[$($attrss)*])*
            pub fn $method_name(&mut self, $field_name: $input_type) -> &mut Self {
                self.$field_name = $field_name;
                self
            }

        };
    }

    // pub(crate) use impl_chainable_setter;
}

/////////////////////////////////////////////////////////////////////////////
#[cfg(feature = "compliance_assert")]
mod compliance_asserts;
#[cfg(feature = "tls")]
pub mod tls;

pub mod callbacks;
pub mod channel;
pub mod connection;
pub mod consumer;
pub mod error;
pub mod security;
