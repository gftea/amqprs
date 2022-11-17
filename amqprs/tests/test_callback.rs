use amqprs::api::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::ExchangeDeclareArguments,
    connection::Connection,
};
use tokio::time;
use tracing::Level;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic = "InternalChannelError(\"channel closed\")"]
async fn test_connection_callback() {
    let _guard = common::setup_logging(Level::TRACE);

    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    // open a channel on the connection
    let channel = connection.open_channel().await.unwrap();

    // expect panic because invalid exchange type will cause server to shutdown connection,
    // the connection callback for `Close` method should be called and all internal channel services are closed
    // which results in error of below API call
    channel
        .exchange_declare(ExchangeDeclareArguments::new("name", "invalid_type"))
        .await
        .unwrap();

    // keep the `channel` and `connection` object from dropping until publish is done
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic = "InternalChannelError(\"channel closed\")"]
async fn test_channel_callback() {
    let _guard = common::setup_logging(Level::TRACE);

    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel().await.unwrap();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap();
    // expect panic because invalid exchange type will cause server to shutdown connection,
    // the connection callback for `Close` method should be called and all internal channel services are closed
    // which results in error of below API call
    channel
        .exchange_declare(ExchangeDeclareArguments::new("amq.topic", "topic"))
        .await
        .unwrap();

    // keep the `channel` and `connection` object from dropping until publish is done
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;
}
