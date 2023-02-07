use bencher::{benchmark_group, benchmark_main, Bencher};

use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicPublishArguments, QueueBindArguments, QueueDeclareArguments, QueuePurgeArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    BasicProperties,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        // .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

fn basic_pub_only(bencher: &mut Bencher) {
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
    let rounting_key = "bench.pub";
    let exchange_name = "amq.topic";
    let queue_name = "bench-q";
    rt.block_on(async {
        // declare a queue
        let (queue_name, _, _) = channel
            .queue_declare(QueueDeclareArguments::new(queue_name))
            .await
            .unwrap()
            .unwrap();

        channel
            .queue_bind(QueueBindArguments::new(
                &queue_name,
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

    let msg_size_list = [0, 1024, connection.frame_max() as usize + 10];

    let task = || async {
        let count = msg_size_list.len();
        for i in 0..count {
            channel
                .basic_publish(
                    BasicProperties::default(),
                    vec![i as u8; msg_size_list[i]],
                    pubargs.clone(),
                )
                .await
                .unwrap();
        }
       
    };
    // start benchmark
    bencher.iter(|| {
        rt.block_on(task());
    });

    // explicitly close
    rt.block_on(async {
        let (_, msg_cnt, _) = channel
            .queue_declare(declargs.clone())
            .await
            .unwrap()
            .unwrap();
        println!("queued message count: {}", msg_cnt);
        // purge queue
        channel
            .queue_purge(QueuePurgeArguments::new(queue_name))
            .await
            .unwrap();
        channel.close().await.unwrap();
        connection.close().await.unwrap();
    })
}

benchmark_group!(basic_pub, basic_pub_only,);

benchmark_main!(basic_pub);
