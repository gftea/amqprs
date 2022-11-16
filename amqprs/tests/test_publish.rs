use amqprs::{
    api::{
        channel::{BasicPublishArguments, ExchangeDeclareArguments},
        connection::Connection,
    },
    BasicProperties,
};
use tokio::time;
use tracing::Level;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish() {
    common::setup_logging(Level::TRACE);

    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel().await.unwrap();

    let exchange_name = "amq.topic";
    let exchange_type = "topic";

    // create arguments for exchange_declare
    let mut args = ExchangeDeclareArguments::new(exchange_name, exchange_type);
    // set to passive mode - checking existence of the exchange
    args.passive = true;
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
    let mut args = BasicPublishArguments::new();
    // set target exchange name
    args.exchange = exchange_name.to_string();
    args.routing_key = "eiffel.a.b.c.d".to_string();
    // basic publish
    channel
        .basic_publish(BasicProperties::default(), content, args)
        .await
        .unwrap();

    // keep the `channel` and `connection` object from dropping until publish is done
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;
}
