use amqprs::{
    api::{
        callbacks::DefaultConnectionCallback,
        channel::{BasicPublishArguments, ExchangeDeclareArguments, ServerSpecificArguments},
        connection::Connection,
    },
    BasicProperties,
};
use tokio::time;
use tracing::Level;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_connection_callback() {
    common::setup_logging(Level::TRACE);

    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap();
    // open a channel on the connection
    let channel = connection.open_channel().await.unwrap();

    // create arguments for exchange_declare
    let exchange_name = "amq.topic";
    let exchange_type = "topic";
    let _args = ExchangeDeclareArguments::new(exchange_name, exchange_type);
    // contents to publish
    let content = String::from("null").into_bytes();
    // create arguments for basic_publish
    let args = BasicPublishArguments::new();

    // Set invalid basic propertities to trigger Close connection from server
    let props = BasicProperties::new(
        Some("err".to_string()),
        Some("err".to_string()),
        Some(ServerSpecificArguments::new()),
        Some(10),
        None,
        None,
        None,
        Some("err".to_string()),
        Some("errid".to_string()),
        Some(1),
        Some("err".to_string()),
        None,
        None,
        None,
    );
    // basic publish
    channel.basic_publish(props, content, args).await.unwrap();

    // keep the `channel` and `connection` object from dropping until publish is done
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;
}
