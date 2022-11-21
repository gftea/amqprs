use self::error::Error;
pub(in crate::api) type Result<T> = std::result::Result<T, Error>;

// macro should appear before module declaration
#[macro_use]
mod helpers {

    macro_rules! synchronous_request {
        ($tx:expr, $msg:expr, $rx:expr, $response:path, $err:path) => {{
            $tx.send($msg).await?;
            match $rx.await? {
                $response(_, method) => Ok(method),
                unexpected => Err($err(unexpected.to_string())),
            }
        }};
    }

    macro_rules! get_expected_method {
        ($frame:expr, $variant:path, $err:expr) => {
            match $frame {
                $variant(_, method) => Ok(method),
                _ => Err($err),
            }
        };
    }
}

/////////////////////////////////////////////////////////////////////////////
mod utils;

pub mod callbacks;
pub mod channel;
pub mod connection;
pub mod consumer;
pub mod error;
