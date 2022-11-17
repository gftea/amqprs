use amqprs::{
    api::{
        channel::{
            BasicConsumeArguments, BasicPublishArguments, Channel, QueueBindArguments,
            QueueDeclareArguments,
        },
        connection::Connection,
        consumer::DefaultConsumer,
    },
    BasicProperties,
};
use tokio::time;
use tracing::Level;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multi_consumer() {
    let _guard = common::setup_logging(Level::DEBUG);

    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    // open a channel on the connection
    let mut channel = connection.open_channel().await.unwrap();

    let exchange_name = "amq.topic";
    // declare a queue
    let queue_name = "amqprs";
    channel
        .queue_declare(QueueDeclareArguments::new(queue_name))
        .await
        .unwrap();

    // bind the queue to exchange
    channel
        .queue_bind(QueueBindArguments::new(
            queue_name,
            exchange_name,
            "eiffel.#",
        ))
        .await
        .unwrap();

    // start consumer with given name
    let mut args = BasicConsumeArguments::new();
    args.queue = queue_name.to_string();
    args.consumer_tag = "amqprs-consumer-example".to_string();

    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // start consumer with generated name by server
    let mut args = BasicConsumeArguments::new();
    args.queue = queue_name.to_string();
    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // recover unacknowledged messages
    // Edge case:
    //  When basic_consume start, we immediately got unacknowledge messages redelivered from servers
    //  before ACK for a message with delivery_tag = 'A' is received by server, meanwhile server receive basic_recover, it will
    //  redeliver the message with different delivery_tag = 'B' and forget about tag 'A',
    //  then once the ACK with delivery_tag = 'A' is received by server later, server will report channel exception.
    //
    // Workaround: wait for all redelivered messages are handled
    time::sleep(time::Duration::from_secs(1)).await;

    channel.basic_recover(true).await.unwrap();

    // publish messages
    publish_test_messages(&channel, exchange_name).await;

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;

    // TODO: move to separate test case, below is for test only
    if false {
        // implicitly close by drop
        drop(channel);
        time::sleep(time::Duration::from_millis(10)).await;
        drop(connection);
        time::sleep(time::Duration::from_millis(10)).await;

    } else {
        // explicitly close
        channel.close().await.unwrap();
        time::sleep(time::Duration::from_millis(10)).await;
        connection.close().await.unwrap();
        time::sleep(time::Duration::from_millis(10)).await;

    }

    
    
}

async fn publish_test_messages(channel: &Channel, exchange_name: &str) {
    // contents to publish
    let content = String::from(
        r#"
            {
                "meta": {"id": "f9d42464-fceb-4282-be95-0cd98f4741b0", "type": "PublishTester", "version": "4.0.0", "time": 1640035100149},
                "data": { "customData": []}, 
                "links": [{"type": "BASE", "target": "fa321ff0-faa6-474e-aa1d-45edf8c99896"}]
            }
        "#
        ).into_bytes();

    // create arguments for basic_publish
    let mut args = BasicPublishArguments::new();
    // set target exchange name
    args.exchange = exchange_name.to_string();
    args.routing_key = "eiffel.a.b.c.d".to_string();

    for _ in 0..10 {
        channel
            .basic_publish(BasicProperties::default(), content.clone(), args.clone())
            .await
            .unwrap();
    }
}
