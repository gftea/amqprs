use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::ExchangeDeclareArguments,
    connection::Connection,
};
use tokio::time;
use tracing::debug;
mod common;

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_net_io_err_handling() {
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

    loop {
        time::sleep(time::Duration::from_secs(1)).await;
        debug!("connection: {}, channel: {}", connection, channel);
        // Manual interruption required:
        // kill tcp stream to see if the loop exit!
        if connection.is_open() == false {
            break;
        }
    }

    // close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
