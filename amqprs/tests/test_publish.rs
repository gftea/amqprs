use amqprs::{
    channel::{BasicPublishArguments, ExchangeDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
    BasicProperties,
};
use tokio::time;
use tracing::Level;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish() {
    let _guard = common::setup_logging(Level::INFO);

    // open a connection to RabbitMQ server
    let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

    let connection = Connection::open(&args).await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();

    let exchange_name = "amq.topic";
    let exchange_type = "topic";

    // create arguments for exchange_declare
    let args = ExchangeDeclareArguments::new(exchange_name, exchange_type)
        .passive(true)
        .finish();

    // declare exchange
    channel.exchange_declare(args).await.unwrap();

    // contents to publish
    let content = String::from(
        r#"
            {
                "meta": {"id": "f9d42464-fceb-4282-be95-0cd98f4741b0", "type": "PublishTester", "version": "4.0.0", "time": 1640035100149},
                "data": { "customData": []}, 
                "links": [{"type": "BASE", "target": "fa321ff0-faa6-474e-aa1d-45edf8c99896"}]}
        "#
        ).into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, exchange_type);

    // basic publish
    channel
        .basic_publish(BasicProperties::default(), content, args)
        .await
        .unwrap();

    // keep the `channel` and `connection` object from dropping until publish is done
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
