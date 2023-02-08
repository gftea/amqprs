use bencher::{benchmark_group, benchmark_main};

///////////////////////////////////////////////////////////////////////////////
/// Common utilities
const ITERATIONS: u64 = 128;

/// We use Fibonacci sequences to generate size list for publish messages
struct Fib {
    max: usize,
    state: usize,
    n_1: usize,
    n_2: usize,
}

impl Fib {
    fn new(max: usize) -> Self {
        assert!(max > 0);
        Self {
            max,
            state: 0,
            n_1: 1,
            n_2: 1,
        }
    }
}

impl Iterator for Fib {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == self.max {
            return None;
        }
        self.state += 1;

        if self.state == 1 || self.state == 2 {
            return Some(1);
        }
        let current = self.n_1 + self.n_2;
        self.n_2 = self.n_1;
        self.n_1 = current;

        Some(current)
    }
}

/// common algorithm for generating size list
fn get_size_list(limit: usize) -> Vec<usize> {
    // construct message size list for publish
    let mut fib = Fib::new(100);
    let mut msg_size_list = vec![0];
    while let Some(v) = fib.next() {
        // println!("{}", v);
        msg_size_list.push(v);
        // avoid OOM error, stop after 1st size > limit
        if v > limit as usize {
            break;
        }
    }
    println!("{:?}", msg_size_list);
    msg_size_list
}

/// common runtime config
fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        // .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// benchmark functions for `amqprs` client
mod client_amqprs {
    use bencher::Bencher;

    use crate::get_size_list;
    use crate::rt;
    use crate::ITERATIONS;

    use amqprs::{
        callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
        channel::{
            BasicPublishArguments, QueueBindArguments, QueueDeclareArguments, QueuePurgeArguments,
        },
        connection::{Connection, OpenConnectionArguments},
        BasicProperties,
    };
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    pub fn amqprs_basic_pub(bencher: &mut Bencher) {
        // construct a subscriber that prints formatted traces to stdout
        // global subscriber with log level according to RUST_LOG
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env())
            .try_init()
            .ok();
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
        let rounting_key = "bench.amqprs.pub";
        let exchange_name = "amq.topic";
        let queue_name = "bench-amqprs-q";
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
        // task to be benchmarked
        let task = || async {
            let count = msg_size_list.len();
            // purge queue
            channel
                .queue_purge(QueuePurgeArguments::new(queue_name))
                .await
                .unwrap();
            let (_, msg_cnt, _) = channel
                .queue_declare(
                    QueueDeclareArguments::new(queue_name)
                        .passive(true)
                        .finish(),
                )
                .await
                .unwrap()
                .unwrap();
            assert_eq!(0, msg_cnt);
            // publish  messages of variable sizes
            for i in 0..count {
                channel
                    .basic_publish(
                        BasicProperties::default(),
                        vec![0xc5; msg_size_list[i]],
                        pubargs.clone(),
                    )
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
        // start benchmark
        bencher.bench_n(ITERATIONS, |b| {
            b.iter(|| {
                rt.block_on(task());
            });
        });

        // explicitly close
        rt.block_on(async {
            channel.close().await.unwrap();
            connection.close().await.unwrap();
        })
    }
}

/// benchmark functions for `lapin` client
mod client_lapin {
    use bencher::Bencher;
    use lapin::{
        options::{BasicPublishOptions, QueueBindOptions, QueueDeclareOptions, QueuePurgeOptions},
        types::FieldTable,
        BasicProperties, Connection, ConnectionProperties,
    };
    use tokio_executor_trait::Tokio;

    use crate::{get_size_list, rt, ITERATIONS};

    pub fn lapin_basic_pub(bencher: &mut Bencher) {
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

        let rounting_key = "bench.lapin.pub";
        let exchange_name = "amq.topic";
        let queue_name = "bench-lapin-q";

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
        let mut declopts = QueueDeclareOptions::default();
        declopts.passive = true;

        let msg_size_list = get_size_list(connection.configuration().frame_max() as usize);

        let task = || async {
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
        };
        // start benchmark
        bencher.bench_n(ITERATIONS, |b| {
            b.iter(|| {
                rt.block_on(task());
            });
        });

        rt.block_on(async {
            channel.close(0, "").await.unwrap();
            connection.close(0, "").await.unwrap();
        })
    }
}

benchmark_group!(amqprs, client_amqprs::amqprs_basic_pub,);
benchmark_group!(lapin, client_lapin::lapin_basic_pub,);

benchmark_main!(amqprs, lapin);
