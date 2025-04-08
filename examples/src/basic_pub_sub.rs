use amqp_serde::types::FieldTable;
use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicConsumeArguments, BasicPublishArguments, QueueBindArguments, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::DefaultConsumer,
    BasicProperties,
};
use tokio::time;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // construct a subscriber that prints formatted traces to stdout
    // global subscriber with log level according to RUST_LOG
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();

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

    // declare a durable queue
    let (queue_name, _, _) = channel
        .queue_declare(QueueDeclareArguments::durable_client_named(
            "amqprs.examples.basic",
        ))
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let routing_key = "amqprs.example";
    let exchange_name = "amq.topic";
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    //////////////////////////////////////////////////////////////////////////////
    // start consumer with given name
    let args = BasicConsumeArguments::new(&queue_name, "example_basic_pub_sub");

    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    //////////////////////////////////////////////////////////////////////////////
    // publish message
    let content = String::from(
        r#"
            {
                "publisher": "example"
                "data": "Hello, amqprs!"
            }
        "#,
    )
    .into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, routing_key);
    let mut headers = FieldTable::new();
    headers.insert("key".try_into().unwrap(), "value".try_into().unwrap());

    let props = BasicProperties::default()
        .with_app_id("app_id")
        .with_cluster_id("cluster_id")
        .with_content_encoding("content_encoding")
        .with_content_type("content_type")
        .with_correlation_id("correlation_id")
        .with_expiration("100000")
        .with_message_id("message_id")
        .with_message_type("message_type")
        .with_persistence(true)
        .with_priority(1)
        .with_reply_to("reply_to")
        .with_timestamp(1743000001)
        .with_user_id("user")
        .with_headers(headers)
        .finish();

    channel.basic_publish(props, content, args).await.unwrap();

    // Check connection should still open and no network i/o failure after publish
    match time::timeout(time::Duration::from_secs(1), connection.listen_network_io_failure()).await {
        Ok(is_failure) => {
            panic!("Unexpected network I/O failure: {is_failure}, connection is_open status: {}", connection.is_open());
        }
        Err(_) => {
            tracing::debug!("Network I/O OK after publish");
            assert!(connection.is_open(), "Connection should be still open");
        }
    }
    // keep the `channel` and `connection` object from dropping before pub/sub is done.
    // channel/connection will be closed when drop.
    time::sleep(time::Duration::from_secs(1)).await;
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
