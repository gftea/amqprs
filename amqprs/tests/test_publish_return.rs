use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::BasicPublishArguments,
    connection::Connection,
    BasicProperties,
};
use tokio::time;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publish_return() {
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
    let exchange_type = "topic";

    // publish with mandatory = true
    // message is not routable due to no queue yet
    let args = BasicPublishArguments::new(exchange_name, exchange_type)
        .mandatory(true)
        .finish();

    channel
        .basic_publish(
            BasicProperties::default(),
            b"undeliverable message".to_vec(),
            args.clone(),
        )
        .await
        .unwrap();
    // wait for publish is done
    time::sleep(time::Duration::from_secs(10)).await;

    // close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
