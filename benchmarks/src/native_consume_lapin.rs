use std::sync::Arc;

use lapin::{
    message::DeliveryResult,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueBindOptions,
        QueueDeclareOptions, QueuePurgeOptions,
    },
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties,
};
use tokio::sync::Notify;
use tokio_executor_trait::Tokio;
mod common;
use common::*;

fn main() {
    setup_tracing();

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

        let rounting_key = "bench.lapin.consume";
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

        let pubopts = BasicPublishOptions::default();
        let declopts = QueueDeclareOptions {
            passive: true,
            ..Default::default()
        };

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

        // publish  messages of variable sizes
        for &i in msg_size_list.iter().take(count) {
            let _confirm = channel
                .basic_publish(
                    exchange_name,
                    rounting_key,
                    pubopts,
                    &vec![0xc5; i],
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

        let consume_opts = BasicConsumeOptions::default();
        let tbl = FieldTable::default();
        let notifyer = Arc::new(Notify::new());
        let notifyee = notifyer.clone();

        //////////////////////////////////////////////////////////////////////////////
        let now = std::time::Instant::now();

        let consumer = channel
            .basic_consume(queue_name, "", consume_opts, tbl)
            .await
            .unwrap();

        consumer.set_delegate(move |delivery: DeliveryResult| {
            let notifyer = notifyer.clone();
            async move {
                let delivery = match delivery {
                    // Carries the delivery alongside its channel
                    Ok(Some(delivery)) => delivery,
                    // The consumer got canceled
                    Ok(None) => return,
                    // Carries the error and is always followed by Ok(None)
                    Err(error) => {
                        dbg!("Failed to consume queue message {}", error);
                        return;
                    }
                };

                if delivery.delivery_tag % count as u64 == 0 {
                    // println!("{} % {}", delivery.delivery_tag, count);
                    let args = BasicAckOptions { multiple: true };
                    delivery.ack(args).await.unwrap();
                    notifyer.notify_one();
                }
            }
        });
        // wait for all messages delivered to consumer
        notifyee.notified().await;
        let eclapsed = now.elapsed();
        println!("lapin consumer benchmarks: {eclapsed:?}");
        //////////////////////////////////////////////////////////////////////////////

        channel.close(0, "").await.unwrap();
        connection.close(0, "").await.unwrap();
    });
}
