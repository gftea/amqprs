use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicGetArguments, BasicPublishArguments, QueueBindArguments,
        QueueDeclareArguments,
    },
    connection::Connection,
    BasicProperties,
};
use tracing::info;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();

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
    let exchange_name = "amq.topic";
    // declare a queue
    let (queue_name, ..) = channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let routing_key = "get.test"; // key should also be used by publish
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // get empty
    let get_args = BasicGetArguments::new(&queue_name);

    // contents to publish
    let content = String::from(
        r#"
            {
                "data": "some data to publish for test"
            }
        "#,
    )
    .into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(&exchange_name, routing_key);

    let num_loop = 3;
    for _ in 0..num_loop {
        channel
            .basic_publish(BasicProperties::default(), content.clone(), args.clone())
            .await
            .unwrap();
    }

    for i in 0..num_loop {
        // get single message
        let delivery_tag = match channel.basic_get(get_args.clone()).await.unwrap() {
            Some((get_ok, basic_props, content)) => {
                #[cfg(feature = "tracing")]
                info!(
                    "Get results:
                    {}
                    {}
                    Content: {}",
                    get_ok,
                    basic_props,
                    std::str::from_utf8(&content).unwrap()
                );
                // message count should decrement accordingly
                assert_eq!(num_loop - 1 - i, get_ok.message_count());
                get_ok.delivery_tag()
            }
            None => panic!("expect get a message"),
        };
        // ack to received message
        channel
            .basic_ack(BasicAckArguments {
                delivery_tag,
                multiple: false,
            })
            .await
            .unwrap();
    }

    // test zero size content
    for _ in 0..num_loop {
        channel
            .basic_publish(BasicProperties::default(), Vec::new(), args.clone())
            .await
            .unwrap();
    }

    for i in 0..num_loop {
        // get single message
        let delivery_tag = match channel.basic_get(get_args.clone()).await.unwrap() {
            Some((get_ok, basic_props, content)) => {
                #[cfg(feature = "tracing")]
                info!(
                    "Get results:
                    {}
                    {}
                    Content: {}",
                    get_ok,
                    basic_props,
                    std::str::from_utf8(&content).unwrap()
                );
                // message count should decrement accordingly
                assert_eq!(num_loop - 1 - i, get_ok.message_count());
                get_ok.delivery_tag()
            }
            None => panic!("expect get a message"),
        };
        // ack to received message
        channel
            .basic_ack(BasicAckArguments {
                delivery_tag,
                multiple: false,
            })
            .await
            .unwrap();
    }

    // test message of size > frame_max
    let body_size = connection.frame_max() as usize + 10;
    for _ in 0..num_loop {
        let content = vec![1; body_size];

        channel
            .basic_publish(BasicProperties::default(), content, args.clone())
            .await
            .unwrap();
    }

    for i in 0..num_loop {
        // get single message
        let delivery_tag = match channel.basic_get(get_args.clone()).await.unwrap() {
            Some((get_ok, _basic_props, content)) => {
                assert_eq!(body_size, content.len());
                // message count should decrement accordingly
                assert_eq!(num_loop - 1 - i, get_ok.message_count());
                get_ok.delivery_tag()
            }
            None => panic!("expect get a message"),
        };
        // ack to received message
        channel
            .basic_ack(BasicAckArguments {
                delivery_tag,
                multiple: false,
            })
            .await
            .unwrap();
    }
    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_empty() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();

    let connection = Connection::open(&args).await.unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();

    let exchange_name = "amq.topic";
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
            exchange_name,
            "__no_one_use_this_key__", // this make sure we receive a empty response
        ))
        .await
        .unwrap();

    // get empty
    let get_message = channel
        .basic_get(BasicGetArguments::new(&queue_name))
        .await
        .unwrap();
    if let Some(_) = get_message {
        panic!("expect ReturnEmpty message");
    }
}
