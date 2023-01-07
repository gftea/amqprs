use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicConsumeArguments, BasicPublishArguments, QueueBindArguments, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::DefaultConsumer,
    BasicProperties,
};
use tokio::{sync::Notify, time};
use tracing::Level;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // open a connection to RabbitMQ server
    let connection = Connection::open(&OpenConnectionArguments::new(
        "localhost",
        5672,
        "user",
        "bitnami",
    ))
    .await
    .unwrap();
    connection
        .register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap();

    // declare a queue
    let (queue_name, _, _) = channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let rounting_key = "amqprs.example";
    let exchange_name = "amq.topic";
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            rounting_key,
        ))
        .await
        .unwrap();

    //////////////////////////////////////////////////////////////////////////////
    // start consumer, auto ack
    let args = BasicConsumeArguments::new(&queue_name, "basic_consumer")
        .no_ack(true)
        .finish();

    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    let guard = Notify::new();
    guard.notified().await;
}
