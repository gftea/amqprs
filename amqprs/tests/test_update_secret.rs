use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    connection::Connection,
};
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_update_secret() {
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

    connection
        .update_secret("123456", "secret expired")
        .await
        .unwrap();

    // close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
