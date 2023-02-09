use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicPublishArguments, QueueBindArguments, QueueDeclareArguments, QueuePurgeArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    BasicProperties,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
///////////////////////////////////////////////////////////////////////////////
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
    for _ in 0..10 {
        msg_size_list.extend_from_within(0..);
    }
    msg_size_list
}

/// common runtime config
fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// benchmark functions for `amqprs` client
fn main() {
    // construct a subscriber that prints formatted traces to stdout
    // global subscriber with log level according to RUST_LOG
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
    let rt = rt();

    rt.block_on(async {
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

        let channel = connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();

        let rounting_key = "bench.amqprs.pub";
        let exchange_name = "amq.topic";
        let queue_name = "bench-amqprs-q";
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

        let pubargs = BasicPublishArguments::new(exchange_name, rounting_key);
        let declargs = QueueDeclareArguments::new(queue_name)
            .passive(true)
            .finish();

        let msg_size_list = get_size_list(connection.frame_max() as usize);
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
        //////////////////////////////////////////////////////////////////////////////
        let now = std::time::Instant::now();
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
        println!("amqprs: {:?}", now.elapsed());
        //////////////////////////////////////////////////////////////////////////////

        channel.close().await.unwrap();
        connection.close().await.unwrap();
    });
}
