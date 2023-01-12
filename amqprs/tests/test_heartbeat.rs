use amqprs::{
    channel::{BasicConsumeArguments, QueueBindArguments, QueueDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
    consumer::DefaultConsumer,
};

use tokio::time;
use tracing::{info, Level};
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_customized_heartbeat() {
    let _guard = common::setup_logging(Level::DEBUG);

    // open a connection to RabbitMQ server,
    let mut args = common::build_conn_args();
    // set heartbeat = 10s
    args.heartbeat(10);

    let connection = Connection::open(&args).await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();

    // declare a queue
    let (queue_name, ..) = channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            "amq.topic",
            "eiffel.#",
        ))
        .await
        .unwrap();

    // start consumer with given name
    let args = BasicConsumeArguments::new(&queue_name, "test_multi_consume");

    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    info!("-------------no heartbeat required--------------------");

    // normal interval is the heartbeat timeout / 2
    let interval: u64 = (connection.heartbeat() / 2).into();
    let num_loop = 5;

    for _ in 0..num_loop {
        time::sleep(time::Duration::from_secs(interval - 1)).await;
        // if any frame sent between heartbeat deadline, the heartbeat deadline will be updated.
        // during this loop, do not expect any heartbeat sent
        channel.flow(true).await.unwrap();
    }

    info!("------------------now heartbeat should be sent as required interval------------------");

    // now heartbeat should be sent as required interval
    time::sleep(time::Duration::from_secs(interval * num_loop)).await;
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
