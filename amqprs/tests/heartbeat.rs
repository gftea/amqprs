use amqprs::{
    channel::{BasicConsumeArguments, QueueBindArguments, QueueDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
    consumer::DefaultConsumer,
};

use tokio::{sync::Notify, time};
use tracing::{Level, info};
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multi_consumer() {
    common::setup_logging(Level::DEBUG).ok();

    // open a connection to RabbitMQ server
    let connection = Connection::open(&OpenConnectionArguments::new(
        "localhost:5672",
        "user",
        "bitnami",
    ))
    .await
    .unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();

    // declare a queue
    let (queue_name, ..) = channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            "amq.topic",
            "eiffel.#",
        ))
        .await
        .unwrap();

    // start consumer with given name
    let args = BasicConsumeArguments::new(&queue_name, "test_multi_consume");

    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();
    info!("------------------------------------------------------");
    
    time::sleep(time::Duration::from_secs(60 * 60)).await;
}
