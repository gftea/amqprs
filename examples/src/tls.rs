use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicConsumeArguments, BasicPublishArguments, QueueBindArguments, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    consumer::DefaultConsumer,
    BasicProperties,
};
use tokio::time;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use amqprs::tls::TlsAdaptor;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // construct a subscriber that prints formatted traces to stdout
    // global subscriber with log level according to RUST_LOG
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();

    ////////////////////////////////////////////////////////////////
    // TLS specific configuration
    let current_dir = std::env::current_dir().unwrap();
    let current_dir = current_dir.join("rabbitmq_conf/client/");

    // domain should match the certificate/key files
    let domain = "AMQPRS_TEST";
    let root_ca_cert = current_dir.join("ca_certificate.pem");
    let client_cert = current_dir.join("client_AMQPRS_TEST_certificate.pem");
    let client_private_key = current_dir.join("client_AMQPRS_TEST_key.pem");
    //
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let args = OpenConnectionArguments::new("localhost", 5671, "user", "bitnami")
        .tls_adaptor(
            TlsAdaptor::with_client_auth(
                Some(root_ca_cert.as_path()),
                client_cert.as_path(),
                client_private_key.as_path(),
                domain.to_owned(),
            )
            .unwrap(),
        )
        .finish();

    ////////////////////////////////////////////////////////////////
    // everything below should be the same as regular connection
    // open a connection to RabbitMQ server
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

    // declare a queue
    let (queue_name, _, _) = channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let rounting_key = "amqprs.example";
    let exchange_name = "amq.topic";
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            rounting_key,
        ))
        .await
        .unwrap();

    //////////////////////////////////////////////////////////////////////////////
    // start consumer with given name
    let args = BasicConsumeArguments::new(&queue_name, "example_basic_pub_sub");

    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    //////////////////////////////////////////////////////////////////////////////
    // publish message
    let content = String::from(
        r#"
            {
                "publisher": "example"
                "data": "Hello, amqprs!"
            }
        "#,
    )
    .into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, rounting_key);

    channel
        .basic_publish(BasicProperties::default(), content, args)
        .await
        .unwrap();

    // keep the `channel` and `connection` object from dropping before pub/sub is done.
    // channel/connection will be closed when drop.
    time::sleep(time::Duration::from_secs(1)).await;
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}
