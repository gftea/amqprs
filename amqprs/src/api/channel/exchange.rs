use amqp_serde::types::{FieldTable, FieldValue};

use crate::{
    api::error::Error,
    frame::{Declare, Frame, Delete},
    net::Response
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
    pub arguments: ServerSpecificArguments,
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
            arguments: ServerSpecificArguments::new(),
        }
    }
}
/// A set of arguments for the declaration.
/// The syntax and semantics of these arguments depends on the server implementation.
#[derive(Debug, Clone)]
pub struct ServerSpecificArguments {
    alternate_exchange: Option<String>,
}

impl ServerSpecificArguments {
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

/////////////////////////////////////////////////////////////////////////////
/// API for Exchange methods
impl Channel {
    pub async fn exchange_declare(&mut self, args: ExchangeDeclareArguments) -> Result<()> {
        let mut declare = Declare::new(
            args.name.try_into().unwrap(),
            args.typ.try_into().unwrap(),
            args.arguments.into_field_table(),
        );
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
        let mut delete = Delete::new(args.name.try_into().unwrap());
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

    pub async fn exchange_bind() {
        
    }

    pub async fn exchange_unbind() {
        
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
