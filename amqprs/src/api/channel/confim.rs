use crate::{
    api::error::Error,
    frame::{Frame, Select, SelectOk},
};

use super::{Channel, Result};

/// Arguments for [`confirm_select`]
///
/// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#confirm.select).
///
/// [`confirm_select`]: struct.Channel.html#method.confirm_select
#[derive(Debug, Clone, Default)]
pub struct ConfirmSelectArguments {
    /// Default: `false`
    pub no_wait: bool,
}

impl ConfirmSelectArguments {
    /// Create new arguments with defaults.
    pub fn new(no_wait: bool) -> Self {
        Self { no_wait }
    }
}

/// APIs for AMQP confirm class.
impl Channel {
    /// See [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#confirm.select).
    ///
    /// # Errors
    ///
    /// Returns error if any failure in comunication with server.
    pub async fn confirm_select(&self, args: ConfirmSelectArguments) -> Result<()> {
        let select = Select::new(args.no_wait);
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, select.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(SelectOk::header()).await?;

            let _method = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, select.into_frame()),
                responder_rx,
                Frame::SelectOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time;

    use crate::{
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        channel::BasicPublishArguments,
        connection::{Connection, OpenConnectionArguments},
        test_utils::setup_logging,
        BasicProperties, DELIVERY_MODE_TRANSIENT,
    };

    use super::ConfirmSelectArguments;

    #[tokio::test]
    async fn test_publish_confirm_mode() {
        setup_logging();

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

        let connection = Connection::open(&args).await.unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();

        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();

        // set to publish confirm mode
        channel
            .confirm_select(ConfirmSelectArguments::default())
            .await
            .unwrap();

        let args = BasicPublishArguments::new("amq.topic", "amqprs.test.transaction");

        let basic_properties = BasicProperties::default()
            .with_persistence(true)
            .finish();

        let content = String::from("AMQPRS test publish confirm").into_bytes();

        channel
            .basic_publish(basic_properties, content, args)
            .await
            .unwrap();

        time::sleep(Duration::from_millis(100)).await;
        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
