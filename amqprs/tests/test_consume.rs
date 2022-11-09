use amqprs::api::{
    channel::{BasicConsumeArguments, QueueBindArguments, QueueDeclareArguments},
    connection::Connection,
    consumer::DefaultConsumer,
};
use tokio::time;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_consume() {
    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    // open a channel on the connection
    let mut channel = connection.open_channel().await.unwrap();

    // declare a queue
    let queue_name = "amqprs";
    channel
        .queue_declare(QueueDeclareArguments::new(queue_name))
        .await
        .unwrap();

    // bind the queue to exchange
    channel
        .queue_bind(QueueBindArguments::new(queue_name, "amq.topic", "eiffel.#"))
        .await
        .unwrap();

    // create arguments for consume
    let mut args = BasicConsumeArguments::new();
    args.queue = queue_name.to_string();
    args.consumer_tag = "amqprs-consumer-example".to_string();

    // start consumer
    channel.basic_consume(args, DefaultConsumer).await.unwrap();

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(60)).await;
}
