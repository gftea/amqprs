//! See [AMQP 0-9-1 extended specs](https://www.rabbitmq.com/resources/specs/amqp0-9-1.extended.xml)
//!
//! The Tx class allows publish and ack operations to be batched into atomic
//! units of work.  The intention is that all publish and ack requests issued
//! within a transaction will complete successfully or none of them will.
//! Servers SHOULD implement atomic transactions at least where all publish
//! or ack requests affect a single queue.  Transactions that cover multiple
//! queues may be non-atomic, given that queues can be created and destroyed
//! asynchronously, and such events do not form part of any transaction.
//! Further, the behaviour of transactions with respect to the immediate and
//! mandatory flags on Basic.Publish methods is not defined.
use crate::{
    api::{error::Error, Result},
    frame::{Frame, TxCommit, TxCommitOk, TxRollback, TxRollbackOk, TxSelect, TxSelectOk},
};

use super::Channel;

/// APIs for AMQP transaction class.
impl Channel {
    /// This method sets the channel to use standard transactions. The client must use this
    /// method at least once on a channel before using the [`tx_commit`] or [`tx_rollback`] methods.
    ///
    /// Also see [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#tx.select).
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    ///
    /// [`tx_commit`]: struct.Channel.html#method.tx_commit
    /// [`tx_rollback`]: struct.Channel.html#method.tx_rollback
    pub async fn tx_select(&self) -> Result<()> {
        let select = TxSelect;

        let responder_rx = self.register_responder(TxSelectOk::header()).await?;

        let _method = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, select.into_frame()),
            responder_rx,
            Frame::TxSelectOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }
    /// This method commits all message publications and acknowledgments performed in
    /// the current transaction.  A new transaction starts immediately after a commit.
    ///
    /// Also see [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#tx.commit).
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    pub async fn tx_commit(&self) -> Result<()> {
        let select = TxCommit;

        let responder_rx = self.register_responder(TxCommitOk::header()).await?;

        let _method = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, select.into_frame()),
            responder_rx,
            Frame::TxCommitOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }
    /// This method abandons all message publications and acknowledgments performed in
    /// the current transaction. A new transaction starts immediately after a rollback.
    /// Note that unacked messages will not be automatically redelivered by rollback;
    /// if that is required an explicit recover call should be issued.
    ///
    /// Also see [AMQP_0-9-1 Reference](https://www.rabbitmq.com/amqp-0-9-1-reference.html#tx.rollback).
    ///
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    pub async fn tx_rollback(&self) -> Result<()> {
        let select = TxRollback;

        let responder_rx = self.register_responder(TxRollbackOk::header()).await?;

        let _method = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, select.into_frame()),
            responder_rx,
            Frame::TxRollbackOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        channel::BasicPublishArguments,
        connection::{Connection, OpenConnectionArguments},
        BasicProperties, DELIVERY_MODE_TRANSIENT,
    };

    #[tokio::test]
    async fn test_tx_apis() {
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

        // start transaction
        channel.tx_select().await.unwrap();

        let args = BasicPublishArguments::new("amq.topic", "amqprs.test.transaction");

        let basic_properties = BasicProperties::default()
            .with_persistence(true)
            .finish();

        let content = String::from("AMQPRS test transactions").into_bytes();

        channel
            .basic_publish(basic_properties, content, args)
            .await
            .unwrap();
        channel.tx_commit().await.unwrap();

        // rollback
        channel.tx_rollback().await.unwrap();

        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
