//! AMQP 0-9-1 client implementation compatible with RabbitMQ.
//!
//! It is currently based on async tokio runtime, and cannot be configured to use another runtime by user.
//!
//! User usually starts by openning an AMQP [`Connection`] and register the connection [`callbacks`],
//! then open an AMQP [`Channel`] on the connection and register the channel [`callbacks`].
//!
//!
//! # Quick Start
//! ```rust
//! use amqprs::connection::{OpenConnectionArguments, Connection};
//! use amqprs::callbacks;
//!
//! # #[tokio::main]
//! # async fn main() {
//! // Build arguments for new connection.
//! let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");
//! // Open an AMQP connection with given arguments.
//! let connection = Connection::open(&args).await.unwrap();
//!
//! // Register connection level callbacks.
//! // In production, user should create its own type and implement trait `ConnectionCallback`.
//! connection.register_callback(callbacks::DefaultConnectionCallback).await.unwrap();
//!
//! // Open an AMQP channel on this connection.
//! let channel = connection.open_channel().await.unwrap();
//! // Register channel level callbacks.
//! // In production, user should create its own type and implement trait `ChannelCallback`.
//! channel.register_callback(callbacks::DefaultChannelCallback).await.unwrap();
//!
//! // ... use the channel or connection ...
//!
//! // gracefully shutdown.
//! channel.close().await.unwrap();
//! connection.close().await.unwrap();
//! # }
//! ```
//!
//! [`Connection`]: connection/struct.Connection.html
//! [`Channel`]: channel/struct.Channel.html
//! [`callbacks`]: callbacks/index.html
/////////////////////////////////////////////////////////////////////////////
mod api;
mod frame;
mod net;

/// public API and types
pub use api::*;
pub use frame::BasicProperties;
pub use frame::Deliver;
pub use frame::GetOk;
pub use frame::Return;
