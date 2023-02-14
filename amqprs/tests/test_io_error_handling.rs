use std::time::Duration;

use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::ExchangeDeclareArguments,
    connection::Connection,
};
use tokio::time;

mod common;

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_net_io_err_handling() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let conn_args = common::build_conn_args();

    let connection = Connection::open(&conn_args).await.unwrap();
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

    // reopen connection
    let handler = || async {
        loop {

            time::sleep(Duration::from_secs(10)).await;
            if let Ok(conn) = Connection::open(&conn_args).await {
                return conn;
            }
        }
    };
    // wait on io failure, and expect new connection created
    let new_conn = connection
        .wait_on_network_io_failure(handler())
        .await
        .unwrap();
    assert_eq!(true, new_conn.is_open());
}
