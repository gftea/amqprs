use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, ExchangeDeclareArguments},
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
                "data": "publish test", 
            }
        "#,
    )
    .into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, "amqprs.example");

    let num_loop = 10000;
    for _ in 0..num_loop {
        // basic publish
        channel
            .basic_publish(BasicProperties::default(), content.clone(), args.clone())
            .await
            .unwrap();
    }
    // wait for publish is done
    time::sleep(time::Duration::from_secs(10)).await;

    // close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
