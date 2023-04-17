use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, ExchangeDeclareArguments, ExchangeType},
    connection::Connection,
    BasicProperties,
};
use tokio::time;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();

    let connection = Connection::open(&args).await.unwrap();
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

    let exchange_name = "amq.topic";

    // create arguments for exchange_declare
    let args = ExchangeDeclareArguments::of_type(exchange_name, ExchangeType::Topic)
        .passive(true)
        .finish();

    // declare exchange
    channel.exchange_declare(args).await.unwrap();

    // contents to publish
    let content = String::from(
        r#"
            {
                "data": "publish test",
            }
        "#,
    )
    .into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, "amqprs.example");

    let num_loop = 5000;
    for _ in 0..num_loop {
        // basic publish
        channel
            .basic_publish(BasicProperties::default(), content.clone(), args.clone())
            .await
            .unwrap();
    }
    for _ in 0..num_loop {
        // publish zero size content
        channel
            .basic_publish(BasicProperties::default(), content.clone(), args.clone())
            .await
            .unwrap();
    }
    // publish message of size > frame_max
    let body_size = connection.frame_max() as usize + 10;
    channel
        .basic_publish(BasicProperties::default(), vec![1; body_size], args.clone())
        .await
        .unwrap();

    // wait for publish is done
    time::sleep(time::Duration::from_secs(10)).await;

    // close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
