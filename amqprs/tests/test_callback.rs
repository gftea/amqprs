use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{ExchangeDeclareArguments, ExchangeType},
    connection::Connection,
};

mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic = "InternalChannelError(\"channel closed\")"]
async fn test_connection_callback() {
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

    // expect panic because invalid exchange type will cause server to shutdown connection,
    // the connection callback for `Close` method should be called and all internal channel services are closed
    // which results in error of below API call
    channel
        .exchange_declare(ExchangeDeclareArguments::new("amq.direct", "invalid_type"))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic = "InternalChannelError(\"channel closed\")"]
async fn test_channel_callback() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();

    let connection = Connection::open(&args).await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();
    channel
        .register_callback(DefaultChannelCallback)
        .await
        .unwrap();

    // expect panic because "amp.topic" is `durable = true`, we declare "durable = false",
    // which is the default value in arguments.
    let args = ExchangeDeclareArguments::of_type("amq.topic", ExchangeType::Topic);
    channel.exchange_declare(args).await.unwrap();
}
