use criterion::{criterion_group, criterion_main, Criterion};
mod common;
use common::*;

/// benchmark functions for `amqprs` client
mod client_amqprs {
    use std::sync::Arc;

    use super::{get_size_list, rt, setup_tracing, BenchMarkConsumer, Criterion};
    use amqprs::{
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        channel::{
            BasicCancelArguments, BasicConsumeArguments, BasicPublishArguments, QueueBindArguments,
            QueueDeclareArguments, QueuePurgeArguments,
        },
        connection::{Connection, OpenConnectionArguments},
        BasicProperties,
    };
    use criterion::BatchSize;
    use tokio::sync::Notify;

    pub fn amqprs_basic_consume(c: &mut Criterion) {
        setup_tracing();

        let rt = rt();

        // open a connection to RabbitMQ server
        let connection = rt.block_on(async {
            let connection = Connection::open(&OpenConnectionArguments::new(
                "localhost",
                5672,
                "user",
                "bitnami",
            ))
            .await
            .unwrap();
            connection
                .register_callback(DefaultConnectionCallback)
                .await
                .unwrap();
            connection
        });

        // open a channel on the connection
        let channel = rt.block_on(async {
            let channel = connection.open_channel(None).await.unwrap();
            channel
                .register_callback(DefaultChannelCallback)
                .await
                .unwrap();
            channel
        });

        //////////////////////////////////////////////////////////////////////////////
        // publish message
        let rounting_key = "bench.amqprs.consume";
        let exchange_name = "amq.topic";
        let queue_name = "bench-amqprs-q";
        let ctag = "ctag-bench-amqprs";

        rt.block_on(async {
            // declare a queue
            let (_, _, _) = channel
                .queue_declare(QueueDeclareArguments::new(queue_name))
                .await
                .unwrap()
                .unwrap();
            // bind queue to exchange
            channel
                .queue_bind(QueueBindArguments::new(
                    queue_name,
                    exchange_name,
                    rounting_key,
                ))
                .await
                .unwrap();
        });

        let pubargs = BasicPublishArguments::new(exchange_name, rounting_key);
        let declargs = QueueDeclareArguments::new(queue_name)
            .passive(true)
            .finish();

        let msg_size_list = get_size_list(connection.frame_max() as usize);
        let count = msg_size_list.len();

        // benchmark setup per iteration
        let setup = || async {
            // cancel consumer
            channel
                .basic_cancel(BasicCancelArguments::new(ctag))
                .await
                .unwrap();
            // purge queue
            channel
                .queue_purge(QueuePurgeArguments::new(queue_name))
                .await
                .unwrap();
            let (_, msg_cnt, consumer_cnt) = channel
                .queue_declare(
                    QueueDeclareArguments::new(queue_name)
                        .passive(true)
                        .finish(),
                )
                .await
                .unwrap()
                .unwrap();
            assert_eq!(0, msg_cnt);
            assert_eq!(0, consumer_cnt);

            // publish  messages of variable sizes

            for &i in msg_size_list.iter().take(count) {
                channel
                    .basic_publish(BasicProperties::default(), vec![0xc5; i], pubargs.clone())
                    .await
                    .unwrap();
            }
            // check all messages arrived at queue
            loop {
                let (_, msg_cnt, _) = channel
                    .queue_declare(declargs.clone())
                    .await
                    .unwrap()
                    .unwrap();

                if count == msg_cnt as usize {
                    break;
                }
            }
        };

        //////////////////////////////////////////////////////////////////////////////
        let notifyer = Arc::new(Notify::new());

        // task to be benchmarked
        let task = || {
            let bench_consumer = BenchMarkConsumer::new(count as u64, notifyer.clone());
            let consume_args = BasicConsumeArguments::new(queue_name, ctag);

            async {
                channel
                    .basic_consume(bench_consumer, consume_args)
                    .await
                    .unwrap();
                // wait for all messages delivered to consumer
                notifyer.notified().await;
            }
        };
        // start benchmark
        c.bench_function("amqprs-basic-consume", |b| {
            b.iter_batched(
                || {
                    rt.block_on(setup());
                },
                |_| {
                    rt.block_on(task());
                },
                BatchSize::PerIteration,
            );
        });
        // explicitly close
        rt.block_on(async {
            channel.close().await.unwrap();
            connection.close().await.unwrap();
        });
    }
}

/// benchmark functions for `lapin` client
mod client_lapin {

    use std::sync::Arc;

    use super::{get_size_list, rt, setup_tracing, Criterion};
    use criterion::BatchSize;
    use lapin::{
        message::DeliveryResult,
        options::{
            BasicAckOptions, BasicCancelOptions, BasicConsumeOptions, BasicPublishOptions,
            QueueBindOptions, QueueDeclareOptions, QueuePurgeOptions,
        },
        types::FieldTable,
        BasicProperties, Connection, ConnectionProperties,
    };
    use tokio::sync::Notify;
    use tokio_executor_trait::Tokio;

    pub fn lapin_basic_consume(c: &mut Criterion) {
        setup_tracing();

        let rt = rt();

        let uri = "amqp://user:bitnami@localhost:5672";
        let options = ConnectionProperties::default()
            // Use tokio executor and reactor.
            // At the moment the reactor is only available for unix.
            .with_executor(Tokio::default().with_handle(rt.handle().clone()))
            .with_reactor(tokio_reactor_trait::Tokio);

        let (connection, channel) = rt.block_on(async {
            let connection = Connection::connect(uri, options).await.unwrap();
            let channel = connection.create_channel().await.unwrap();

            (connection, channel)
        });

        let rounting_key = "bench.lapin.consume";
        let exchange_name = "amq.topic";
        let queue_name = "bench-lapin-q";
        let ctag = "ctag-bench-lapin";

        rt.block_on(async {
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
        });

        let pubopts = BasicPublishOptions::default();
        let declopts = QueueDeclareOptions {
            passive: true,
            ..Default::default()
        };

        let msg_size_list = get_size_list(connection.configuration().frame_max() as usize);
        let count = msg_size_list.len();

        // benchmark setup per iteration
        let setup = || async {
            // cancel consumer
            channel
                .basic_cancel(ctag, BasicCancelOptions::default())
                .await
                .unwrap();
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
            assert_eq!(0, q_state.consumer_count());
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
        };

        //////////////////////////////////////////////////////////////////////////////
        let notifyer = Arc::new(Notify::new());

        // task to benchmark
        let task = || async {
            let consumer = channel
                .basic_consume(
                    queue_name,
                    ctag,
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .unwrap();
            let notifyer = notifyer.clone();
            let notifyee = notifyer.clone();
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
        };

        // start benchmark
        c.bench_function("lapin-basic-consume", |b| {
            b.iter_batched(
                || {
                    rt.block_on(setup());
                },
                |_| {
                    rt.block_on(task());
                },
                BatchSize::PerIteration,
            )
        });

        rt.block_on(async {
            channel.close(0, "").await.unwrap();
            connection.close(0, "").await.unwrap();
        });
    }
}

criterion_group! {
    name = basic_consume;
    config = Criterion::default();
    targets = client_amqprs::amqprs_basic_consume,  client_lapin::lapin_basic_consume
}

criterion_main!(basic_consume);
