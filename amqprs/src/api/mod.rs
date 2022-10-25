// macro should appear before module declaration
#[macro_use]
mod helpers {

    macro_rules! synchronous_request {
        ($tx:expr, $msg:expr, $rx:expr, $response:path, $err:path) => {{
            $tx.send($msg).await.map_err(|err| crate::api::error::Error::InternalChannelError("send error".to_string()))?;
            match $rx
                .recv()
                .await
                .ok_or_else(|| Error::InternalChannelError("receive error".to_string()))?
            {
                crate::net::IncomingMessage::Ok(frame) => match frame {
                    $response(_, method) => Ok(method),
                    unexpected => Err($err(unexpected.to_string())),
                },
                crate::net::IncomingMessage::Exception(error_code, error_msg) => {
                    Err($err(format!("{}: {}", error_code, error_msg)))
                }
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
pub mod channel;
pub mod connection;
pub mod error;
