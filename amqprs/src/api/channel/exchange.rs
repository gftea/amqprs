use crate::{
    api::error::Error,
    frame::{Bind, BindOk, Declare, DeclareOk, Delete, DeleteOk, Frame, Unbind, UnbindOk},
};

use super::{AmqArgumentTable, Channel, Result};

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
    pub arguments: AmqArgumentTable,
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
            arguments: AmqArgumentTable::new(),
        }
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
    pub arguments: AmqArgumentTable,
}

impl ExchangeBindArguments {
    /// Create arguments with defaults
    pub fn new(destination: &str, source: &str, routing_key: &str) -> Self {
        Self {
            destination: destination.to_string(),
            source: source.to_string(),
            routing_key: routing_key.to_string(),
            no_wait: false,
            arguments: AmqArgumentTable::new(),
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
    pub arguments: AmqArgumentTable,
}

impl ExchangeUnbindArguments {
    /// Create arguments with defaults
    pub fn new(destination: &str, source: &str, routing_key: &str) -> Self {
        Self {
            destination: destination.to_string(),
            source: source.to_string(),
            routing_key: routing_key.to_string(),
            no_wait: false,
            arguments: AmqArgumentTable::new(),
        }
    }
}
/////////////////////////////////////////////////////////////////////////////
/// API for Exchange methods
impl Channel {
    pub async fn exchange_declare(&self, args: ExchangeDeclareArguments) -> Result<()> {
        let mut declare = Declare::new(
            0,
            args.name.try_into().unwrap(),
            args.typ.try_into().unwrap(),
            0,
            args.arguments.into_field_table(),
        );

        declare.set_passive(args.passive);
        declare.set_durable(args.durable);
        declare.set_auto_delete(args.auto_delete);
        declare.set_internal(args.internal);
        declare.set_no_wait(args.no_wait);

        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, declare.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(DeclareOk::header()).await?;

            let _method = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, declare.into_frame()),
                responder_rx,
                Frame::DeclareOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }

    pub async fn exchange_delete(&self, args: ExchangeDeleteArguments) -> Result<()> {
        let mut delete = Delete::new(0, args.name.try_into().unwrap(), 0);
        delete.set_if_unused(args.if_unused);
        delete.set_no_wait(args.no_wait);
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, delete.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(DeleteOk::header()).await?;

            let _method = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, delete.into_frame()),
                responder_rx,
                Frame::DeleteOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }

    pub async fn exchange_bind(&self, args: ExchangeBindArguments) -> Result<()> {
        let bind = Bind::new(
            0,
            args.destination.try_into().unwrap(),
            args.source.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.no_wait,
            args.arguments.into_field_table(),
        );
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, bind.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(BindOk::header()).await?;

            synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, bind.into_frame()),
                responder_rx,
                Frame::BindOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }

    pub async fn exchange_unbind(&self, args: ExchangeUnbindArguments) -> Result<()> {
        let unbind = Unbind::new(
            0,
            args.destination.try_into().unwrap(),
            args.source.try_into().unwrap(),
            args.routing_key.try_into().unwrap(),
            args.no_wait,
            args.arguments.into_field_table(),
        );
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, unbind.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(UnbindOk::header()).await?;

            synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, unbind.into_frame()),
                responder_rx,
                Frame::UnbindOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExchangeDeclareArguments, ExchangeDeleteArguments};
    use crate::api::connection::{Connection, OpenConnectionArguments};

    #[tokio::test]
    async fn test_exchange_declare() {
        let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

        let client = Connection::open(&args).await.unwrap();

        let channel = client.open_channel().await.unwrap();
        let mut args = ExchangeDeclareArguments::new("amq.direct", "direct");
        args.passive = true;
        channel.exchange_declare(args).await.unwrap();
    }

    #[tokio::test]
    #[should_panic = "InternalChannelError(\"channel closed\")"]
    async fn test_exchange_delete() {
        let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

        let client = Connection::open(&args).await.unwrap();

        let channel = client.open_channel().await.unwrap();
        let args = ExchangeDeleteArguments::new("amq.direct");
        channel.exchange_delete(args).await.unwrap();
    }
}
