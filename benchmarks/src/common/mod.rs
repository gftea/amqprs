use std::sync::Arc;

use amqprs::{
    channel::{BasicAckArguments, Channel},
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};
use tokio::sync::Notify;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// We use Fibonacci sequences to generate size list for publish messages
pub struct Fib {
    max: usize,
    state: usize,
    n_1: usize,
    n_2: usize,
}

impl Fib {
    pub fn new(max: usize) -> Self {
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
pub fn get_size_list(limit: usize) -> Vec<usize> {
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
pub fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

pub fn setup_tracing() {
    // construct a subscriber that prints formatted traces to stdout
    // global subscriber with log level according to RUST_LOG
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
}

pub struct BenchMarkConsumer {
    end_tag: u64,
    notify: Arc<Notify>,
}

impl BenchMarkConsumer {
    pub fn new(end_tag: u64, notify: Arc<Notify>) -> Self {
        Self { end_tag, notify }
    }
}

#[async_trait::async_trait]
impl AsyncConsumer for BenchMarkConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {
        // check all messages received
        if deliver.delivery_tag() % self.end_tag == 0 {
            // println!("{} % {}", deliver.delivery_tag(), self.end_tag);
            channel
                .basic_ack(BasicAckArguments::new(deliver.delivery_tag(), true))
                .await
                .unwrap();
            self.notify.notify_one();
        }
    }
}
