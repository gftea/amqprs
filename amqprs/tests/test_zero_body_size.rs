use amqp_serde::types::FieldTable;
use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicCancelArguments, BasicConsumeArguments, BasicPublishArguments, Channel,
        QueueBindArguments, QueueDeclareArguments,
    },
    connection::Connection,
    consumer::{DefaultBlockingConsumer, DefaultConsumer},
    BasicProperties, DELIVERY_MODE_TRANSIENT,
};
use tokio::time;
mod common;

#[tokio::test]
async fn test_zero_body_size() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();
    let connection = Connection::open(&args).await.unwrap();

    // open a channel dedicated for consumer on the connection
    let consumer_channel = connection.open_channel(None).await.unwrap();

    let exchange_name = "amq.topic";
    // declare a queue
    let (queue_name, ..) = consumer_channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let routing_key = file!();
    consumer_channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // start consumer with generated name by server
    let args = BasicConsumeArguments::new(&queue_name, "");
    consumer_channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // open a channel dedicated for publisher on the connection
    let pub_channel = connection.open_channel(None).await.unwrap();
    // zero body size content
    let content = vec![];

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, routing_key);

    // basic publish
    pub_channel
        .basic_publish(BasicProperties::default(), content.clone(), args.clone())
        .await
        .unwrap();

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;

    // explicitly close
    pub_channel.close().await.unwrap();
    consumer_channel.close().await.unwrap();
    connection.close().await.unwrap();
}
