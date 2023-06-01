//! AMQP 0-9-1 client implementation compatible with RabbitMQ.
//!
//! It is currently based on async tokio runtime, and cannot be configured to use another runtime by user.
//!
//! User usually starts by openning an AMQP [`Connection`] and register the connection [`callbacks`],
//! then open an AMQP [`Channel`] on the connection and register the channel [`callbacks`].
//!
//!
//! # Quick Start
//!
//! ```rust
//! use amqprs::{
//!     callbacks,
//!     security::SecurityCredentials,
//!     connection::{OpenConnectionArguments, Connection},
//! };
//!
//! # #[tokio::main]
//! # async fn main() {
//! // Build arguments for new connection.
//! let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");
//! // Open an AMQP connection with given arguments.
//! let connection = Connection::open(&args).await.unwrap();
//!
//! // Register connection level callbacks.
//! // In production, user should create its own type and implement trait `ConnectionCallback`.
//! connection.register_callback(callbacks::DefaultConnectionCallback).await.unwrap();
//!
//! // ... Now, ready to use the connection ...
//!
//! // Open an AMQP channel on this connection.
//! let channel = connection.open_channel(None).await.unwrap();
//! // Register channel level callbacks.
//! // In production, user should create its own type and implement trait `ChannelCallback`.
//! channel.register_callback(callbacks::DefaultChannelCallback).await.unwrap();
//!
//! // ... Now, ready to use the channel ...
//! // For examples:
//! channel.flow(true).await.unwrap();
//!
//! // gracefully shutdown.
//! channel.close().await.unwrap();
//! connection.close().await.unwrap();
//! # }
//! ```
//!
//! # Optional Features
//!
//! - "traces": enable `tracing` in the library.
//! - "compliance_assert": enable compliance assertion according to AMQP spec.
//!     If enabled, library always check user inputs and `panic` if any non-compliance.
//!     If disabled, then it relies on server to reject.
//! - "tls": enable SSL/TLS.
//! - "urispec": enable support of [RabbitMQ URI Specification](https://www.rabbitmq.com/uri-spec.html)
//!
//! [`Connection`]: connection/struct.Connection.html
//! [`Channel`]: channel/struct.Channel.html
//! [`callbacks`]: callbacks/index.html
//!
/////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod test_utils;

/////////////////////////////////////////////////////////////////////////////
mod api;
mod frame;
mod net;

/// public API and types
pub use api::*;
pub use frame::Ack;
pub use frame::BasicProperties;
pub use frame::Cancel;
pub use frame::Close;
pub use frame::CloseChannel;
pub use frame::Deliver;
pub use frame::GetOk;
pub use frame::Nack;
pub use frame::Return;

pub use frame::DELIVERY_MODE_PERSISTENT;
pub use frame::DELIVERY_MODE_TRANSIENT;

pub use amqp_serde::types::*;
