use amqp_serde::types::{FieldTable, FieldValue, ShortUint};

use crate::{
    api::error::Error,
    frame::{Bind, Declare, Delete, Frame, Unbind},
    net::Response,
};

use super::{Channel, Result};

/// Arguments for [`exchange_declare`]
///
/// [`exchange_declare`]: crate::api::channel::Channel::exchange_declare
#[derive(Debug, Clone)]
pub struct ExchangeDeclareArguments {
    pub name: String,
    pub typ: String,
    pub passive: bool,
    pub durable: bool,
    pub auto_delete: bool,
    pub internal: bool,
    pub no_wait: bool,
    pub arguments: DeclareExtensionArguments,
}

impl ExchangeDeclareArguments {
    /// Create declare arguments with defaults
    pub fn new(name: &str, typ: &str) -> Self {
        Self {
            name: name.to_string(),
            typ: typ.to_string(),
            passive: false,
            durable: false,
            auto_delete: false,
            internal: false,
            no_wait: false,
            arguments: DeclareExtensionArguments::new(),
        }
    }
}
/// A set of arguments for the declaration.
/// The syntax and semantics of these arguments depends on the server implementation.
#[derive(Debug, Clone)]
pub struct DeclareExtensionArguments {
    alternate_exchange: Option<String>,
}

impl DeclareExtensionArguments {
    pub fn new() -> Self {
        Self {
            alternate_exchange: None,
        }
    }
    /// RabbitMQ server's feature [`Alternate Exchange`]
    ///
    /// [`Alternate Exchange`]: https://www.rabbitmq.com/ae.html
    pub fn set_alternate_exchange(&mut self, alternate_exchange: String) {
        self.alternate_exchange = Some(alternate_exchange);
    }

    // the field table type should be hidden from API
    fn into_field_table(self) -> FieldTable {
        let mut table = FieldTable::new();
        if let Some(alt_exchange) = self.alternate_exchange {
            table.insert(
                "alternate_exchange".try_into().unwrap(),
                FieldValue::S(alt_exchange.try_into().unwrap()),
            );
        }

        table
    }
}

/// Arguments for [`exchange_delete`]
///
/// [`exchange_delete`]: crate::api::channel::Channel::exchange_delete
#[derive(Debug, Clone)]
pub struct ExchangeDeleteArguments {
    pub name: String,
    pub if_unused: bool,
    pub no_wait: bool,
}

impl ExchangeDeleteArguments {
    /// Create arguments with defaults
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            if_unused: false,
            no_wait: false,
        }
    }
}

/// Arguments for [`exchange_bind`]
///
/// [`exchange_bind`]: crate::api::channel::Channel::exchange_bind
#[derive(Debug, Clone)]
pub struct ExchangeBindArguments {
    pub destination: String,
    pub source: String,
    pub routing_key: String,
    pub no_wait: bool,
    /// A set of arguments for the binding.
    /// The syntax and semantics of these arguments depends on the exchange class
    /// What is accepted arguments?
    pub arguments: BindExtentionArguments,
}

#[derive(Debug, Clone)]
pub struct BindExtentionArguments {}
impl BindExtentionArguments {
    pub fn new() -> Self {
        Self {}
    }
    fn into_field_table(self) -> FieldTable {
        let mut table = FieldTable::new();
        table
    }
}

impl ExchangeBindArguments {
    /// Create arguments with defaults
    pub fn new(destination: &str, source: &str, routing_key: &str) -> Self {
        Self {
            destination: destination.to_string(),
            source: source.to_string(),
            routing_key: routing_key.to_string(),
            no_wait: false,
            arguments: BindExtentionArguments::new(),
        }
    }
}

/// Arguments for [`exchange_unbind`]
///
/// [`exchange_unbind`]: crate::api::channel::Channel::exchange_unbind
#[derive(Debug, Clone)]
pub struct ExchangeUnbindArguments {
    pub destination: String,
    pub source: String,
    pub routing_key: String,
    pub no_wait: bool,
    /// A set of arguments for the Unbinding.
    /// The syntax and semantics of these arguments depends on the exchange class
    /// What is accepted arguments?
    pub arguments: UnbindExtentionArguments,
}

#[derive(Debug, Clone)]
pub struct UnbindExtentionArguments {}
impl UnbindExtentionArguments {
    pub fn new() -> Self {
        Self {}
    }
    fn into_field_table(self) -> FieldTable {
        let mut table = FieldTable::new();
        table
    }
}

impl ExchangeUnbindArguments {
    /// Create arguments with defaults
    pub fn new(destination: &str, source: &str, routing_key: &str) -> Self {
        Self {
            destination: destination.to_string(),
            source: source.to_string(),
            routing_key: routing_key.to_string(),
            no_wait: false,
            arguments: UnbindExtentionArguments::new(),
        }
    }
}
/////////////////////////////////////////////////////////////////////////////
/// API for Exchange methods
impl Channel {
    pub async fn exchange_declare(&mut self, args: ExchangeDeclareArguments) -> Result<()> {
        let mut declare = Declare {
            ticket: 0,
            exchange: args.name.try_into().unwrap(),
            typ: args.typ.try_into().unwrap(),
            bits: 0,
            arguments: args.arguments.into_field_table(),
        };

        declare.set_passive(args.passive);
        declare.set_durable(args.durable);
        declare.set_auto_delete(args.auto_delete);
        declare.set_internal(args.internal);
        declare.set_no_wait(args.no_wait);

        if args.no_wait {
            self.tx
                .send((self.channel_id, declare.into_frame()))
                .await?;
            Ok(())
        } else {
            synchronous_request!(
                self.tx,
                (self.channel_id, declare.into_frame()),
                self.rx,
                Frame::DeclareOk,
                (),
                Error::ChannelUseError
            )
        }
    }

    pub async fn exchange_delete(&mut self, args: ExchangeDeleteArguments) -> Result<()> {
        let mut delete = Delete {
            ticket: 0,
            exchange: args.name.try_into().unwrap(),
            bits: 0,
        };
        delete.set_if_unused(args.if_unused);
        delete.set_no_wait(args.no_wait);
        if args.no_wait {
            self.tx.send((self.channel_id, delete.into_frame())).await?;
            Ok(())
        } else {
            synchronous_request!(
                self.tx,
                (self.channel_id, delete.into_frame()),
                self.rx,
                Frame::DeleteOk,
                (),
                Error::ChannelUseError
            )
        }
    }

    pub async fn exchange_bind(&mut self, args: ExchangeBindArguments) -> Result<()> {
        let bind = Bind {
            ticket: 0,
            destination: args.destination.try_into().unwrap(),
            source: args.source.try_into().unwrap(),
            routing_key: args.routing_key.try_into().unwrap(),
            nowait: args.no_wait,
            arguments: args.arguments.into_field_table(),
        };
        if args.no_wait {
            self.tx.send((self.channel_id, bind.into_frame())).await?;
            Ok(())
        } else {
            synchronous_request!(
                self.tx,
                (self.channel_id, bind.into_frame()),
                self.rx,
                Frame::BindOk,
                (),
                Error::ChannelUseError
            )
        }
    }

    pub async fn exchange_unbind(&mut self, args: ExchangeUnbindArguments) -> Result<()> {
        let unbind = Unbind {
            ticket: 0,
            destination: args.destination.try_into().unwrap(),
            source: args.source.try_into().unwrap(),
            routing_key: args.routing_key.try_into().unwrap(),
            nowait: args.no_wait,
            arguments: args.arguments.into_field_table(),
        };
        if args.no_wait {
            self.tx.send((self.channel_id, unbind.into_frame())).await?;
            Ok(())
        } else {
            synchronous_request!(
                self.tx,
                (self.channel_id, unbind.into_frame()),
                self.rx,
                Frame::UnbindOk,
                (),
                Error::ChannelUseError
            )
        }

    }
}

#[cfg(test)]
mod tests {
    use super::{ExchangeDeclareArguments, ExchangeDeleteArguments};
    use crate::api::connection::Connection;

    #[tokio::test]
    async fn test_exchange_declare() {
        let mut client = Connection::open("localhost:5672").await.unwrap();

        let mut channel = client.channel().await.unwrap();
        let mut args = ExchangeDeclareArguments::new("amq.direct", "direct");
        args.passive = true;
        channel.exchange_declare(args).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_exchange_delete() {
        let mut client = Connection::open("localhost:5672").await.unwrap();

        let mut channel = client.channel().await.unwrap();
        let mut args = ExchangeDeleteArguments::new("amq.direct");
        channel.exchange_delete(args).await.unwrap();
    }
}
