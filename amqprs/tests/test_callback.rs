use amqprs::{
    callbacks::{ChannelCallback, DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicConsumeArguments, Channel, ExchangeDeclareArguments, ExchangeType,
        QueueDeclareArguments, QueueDeleteArguments,
    },
    connection::Connection,
    consumer::DefaultConsumer,
    error::Error,
    Ack, BasicProperties, Cancel, CloseChannel, Nack, Return,
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;

mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic = "InternalChannelError(\"channel closed\")"]
async fn test_connection_callback() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();

    let connection = Connection::open(&args).await.unwrap();

    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();

    // expect panic because invalid exchange type will cause server to shutdown connection,
    // the connection callback for `Close` method should be called and all internal channel services are closed
    // which results in error of below API call
    channel
        .exchange_declare(ExchangeDeclareArguments::new("amq.direct", "invalid_type"))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic = "InternalChannelError(\"channel closed\")"]
async fn test_channel_callback() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();

    let connection = Connection::open(&args).await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap();

    // expect panic because "amp.topic" is `durable = true`, we declare "durable = false",
    // which is the default value in arguments.
    let args = ExchangeDeclareArguments::of_type("amq.topic", ExchangeType::Topic);
    channel.exchange_declare(args).await.unwrap();
}

#[tokio::test]
async fn test_channel_callback_close() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();
    let connection = Connection::open(&args).await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();
    let cancel_notify = Arc::new(Notify::new());
    channel
        .register_callback(ChannelCancelCallback {
            cancel_notify: Arc::clone(&cancel_notify),
        })
        .await
        .unwrap();

    // declare queue
    const QUEUE_NAME: &str = "test-channel-callback-close";
    let args = QueueDeclareArguments::default()
        .queue(QUEUE_NAME.to_string())
        .finish();
    channel.queue_declare(args).await.unwrap();

    // start consumer
    let args = BasicConsumeArguments::default()
        .queue(QUEUE_NAME.to_string())
        .finish();
    channel
        .basic_consume(DefaultConsumer::new(true), args)
        .await
        .unwrap();

    // delete queue so RabbitMQ produce consumer cancel notification
    let args = QueueDeleteArguments::default()
        .queue(QUEUE_NAME.to_string())
        .finish();
    channel.queue_delete(args).await.unwrap();

    // wait for callback
    tokio::time::timeout(Duration::from_secs(5), cancel_notify.notified())
        .await
        .expect("ChannelCallback::cancel() should be called when queue was deleted");
}

struct ChannelCancelCallback {
    cancel_notify: Arc<Notify>,
}

#[async_trait]
impl ChannelCallback for ChannelCancelCallback {
    async fn close(&mut self, _channel: &Channel, _close: CloseChannel) -> Result<(), Error> {
        Ok(())
    }
    async fn cancel(&mut self, _channel: &Channel, _cancel: Cancel) -> Result<(), Error> {
        self.cancel_notify.notify_one();

        Ok(())
    }
    async fn flow(&mut self, _channel: &Channel, active: bool) -> Result<bool, Error> {
        Ok(active)
    }
    async fn publish_ack(&mut self, _channel: &Channel, _ack: Ack) {}
    async fn publish_nack(&mut self, _channel: &Channel, _nack: Nack) {}
    async fn publish_return(
        &mut self,
        _channel: &Channel,
        _ret: Return,
        _basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {
    }
}
