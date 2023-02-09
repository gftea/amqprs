use lapin::{
    options::{BasicPublishOptions, QueueBindOptions, QueueDeclareOptions, QueuePurgeOptions},
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties,
};
use tokio_executor_trait::Tokio;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
mod common;
use common::*;


fn main() {
    // construct a subscriber that prints formatted traces to stdout
    // global subscriber with log level according to RUST_LOG
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
    let rt = rt();
    let options = ConnectionProperties::default()
        // Use tokio executor and reactor.
        // At the moment the reactor is only available for unix.
        .with_executor(Tokio::default().with_handle(rt.handle().clone()))
        .with_reactor(tokio_reactor_trait::Tokio);

    rt.block_on(async {
        let uri = "amqp://user:bitnami@localhost:5672";
        let connection = Connection::connect(uri, options).await.unwrap();
        let channel = connection.create_channel().await.unwrap();

        let rounting_key = "bench.lapin.pub";
        let exchange_name = "amq.topic";
        let queue_name = "bench-lapin-q";

        channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();
        channel
            .queue_bind(
                queue_name,
                exchange_name,
                rounting_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();
        channel
            .queue_bind(
                queue_name,
                exchange_name,
                rounting_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        let pubopts = BasicPublishOptions::default();
        let mut declopts = QueueDeclareOptions::default();
        declopts.passive = true;

        let msg_size_list = get_size_list(connection.configuration().frame_max() as usize);

        let count = msg_size_list.len();
        // purge queue
        channel
            .queue_purge(queue_name, QueuePurgeOptions::default())
            .await
            .unwrap();
        let q_state = channel
            .queue_declare(queue_name, declopts, FieldTable::default())
            .await
            .unwrap();

        assert_eq!(0, q_state.message_count());

        //////////////////////////////////////////////////////////////////////////////
        let now = std::time::Instant::now();

        // publish  messages of variable sizes
        for i in 0..count {
            let _confirm = channel
                .basic_publish(
                    exchange_name,
                    rounting_key,
                    pubopts,
                    &vec![0xc5; msg_size_list[i]],
                    BasicProperties::default(),
                )
                .await
                .unwrap()
                .await
                .unwrap();
        }
        // check all messages arrived at queue
        loop {
            let q_state = channel
                .queue_declare(queue_name, declopts, FieldTable::default())
                .await
                .unwrap();
            if count == q_state.message_count() as usize {
                break;
            }
        }
        println!("lapin benchmarks: {:?}", now.elapsed());
        //////////////////////////////////////////////////////////////////////////////

        channel.close(0, "").await.unwrap();
        connection.close(0, "").await.unwrap();
    });
}
