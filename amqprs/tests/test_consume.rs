use amqp_serde::types::FieldTable;
use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicCancelArguments, BasicConsumeArguments, BasicPublishArguments, Channel,
        QueueBindArguments, QueueDeclareArguments,
    },
    connection::Connection,
    consumer::{DefaultBlockingConsumer, DefaultConsumer},
    BasicProperties, DELIVERY_MODE_TRANSIENT,
};
use tokio::time;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_multi_consumer() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();
    let connection = Connection::open(&args).await.unwrap();

    // open a channel dedicated for consumer on the connection
    let consumer_channel = connection.open_channel(None).await.unwrap();

    let exchange_name = "amq.topic";
    // declare a queue
    let (queue_name, ..) = consumer_channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let routing_key = "amqprs_test_multi_consumer";
    consumer_channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // start consumer with given name
    let args = BasicConsumeArguments::new(&queue_name, "amqprs_test_multi_consumer");

    consumer_channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // start consumer with generated name by server
    let args = BasicConsumeArguments::new(&queue_name, "");
    consumer_channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // open a channel dedicated for publisher on the connection
    let pub_channel = connection.open_channel(None).await.unwrap();
    // publish test messages
    publish_test_messages(&pub_channel, exchange_name, routing_key, 100).await;

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;

    // explicitly close
    pub_channel.close().await.unwrap();
    consumer_channel.close().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_blocking_consumer() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();
    let connection = Connection::open(&args).await.unwrap();

    // open a channel dedicated for consumer on the connection
    let consumer_channel = connection.open_channel(None).await.unwrap();

    let exchange_name = "amq.topic";
    // declare a queue
    let (queue_name, ..) = consumer_channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let routing_key = "amqprs_test_blocking_consumer";
    consumer_channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // start blocking consumer
    let args = BasicConsumeArguments::new(&queue_name, "amqprs_test_blocking_consumer");

    consumer_channel
        .basic_consume_blocking(DefaultBlockingConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // open a channel dedicated for publisher on the connection
    let pub_channel = connection.open_channel(None).await.unwrap();
    // publish test messages
    publish_test_messages(&pub_channel, exchange_name, routing_key, 5).await;

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;

    // explicitly close
    pub_channel.close().await.unwrap();
    consumer_channel.close().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_consumer_rx() {
    common::setup_logging();

    // open a connection to RabbitMQ server
    let args = common::build_conn_args();
    let connection = Connection::open(&args).await.unwrap();

    // open a channel dedicated for consumer on the connection
    let consumer_channel = connection.open_channel(None).await.unwrap();

    let exchange_name = "amq.topic";
    // declare a queue
    let (queue_name, ..) = consumer_channel
        .queue_declare(QueueDeclareArguments::default())
        .await
        .unwrap()
        .unwrap();

    // bind the queue to exchange
    let routing_key = "amqprs_test_consumer_rx";
    consumer_channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // start blocking consumer
    let args = BasicConsumeArguments::new(&queue_name, "amqprs_test_consumer_rx");

    let (ctag, mut rx) = consumer_channel.basic_consume_rx(args).await.unwrap();

    tokio::spawn(async move { while let Some(_msg) = rx.recv().await {} });

    // open a channel dedicated for publisher on the connection
    let pub_channel = connection.open_channel(None).await.unwrap();
    // publish test messages
    publish_test_messages(&pub_channel, exchange_name, routing_key, 5).await;

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;

    // explicitly close
    pub_channel.close().await.unwrap();
    consumer_channel
        .basic_cancel(BasicCancelArguments::new(&ctag))
        .await
        .unwrap();
    consumer_channel.close().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_consume_redelivered_messages() {
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
    let routing_key = "amqprs_test_consume_redelivered";
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // server will redeliver the unacknowledged messages to the consumer immediately
    // when consumer starts, to simulate this behavior, publish the messages to server
    // before consumer starts.

    // publish messages
    let num_of_message = 10;
    publish_test_messages(&channel, exchange_name, routing_key, num_of_message).await;
    // wait for publish is done.
    time::sleep(time::Duration::from_secs(1)).await;

    // verify the queue has the right message count
    let args = QueueDeclareArguments::new(&queue_name)
        .passive(true)
        .finish();
    let (_, message_count, consumer_count) =
        channel.queue_declare(args.clone()).await.unwrap().unwrap();
    assert_eq!(num_of_message, message_count as usize);
    assert_eq!(0, consumer_count);

    // start consumer
    channel
        .basic_consume(
            DefaultConsumer::new(false),
            BasicConsumeArguments::new(&queue_name, ""),
        )
        .await
        .unwrap();
    // wait for consume is done
    time::sleep(time::Duration::from_secs(1)).await;

    // verify all messages are consumed.
    let (_, message_count, consumer_count) =
        channel.queue_declare(args.clone()).await.unwrap().unwrap();
    assert_eq!(0, message_count);
    assert_eq!(1, consumer_count);

    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_cancel_consumer() {
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
    let routing_key = "amqprs_test_cancel_consumer";
    channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // start consumer with auto-ack.
    // NOTE: use automatic ack, because if running both publisher and
    // manual-ACK consumer on same channel cocurrently, the ACK frames
    // may interleave with the publish sequence which results in server
    // exception to close the connection.
    let consumer_tag = channel
        .basic_consume(
            DefaultConsumer::new(true),
            BasicConsumeArguments::new(&queue_name, ""),
        )
        .await
        .unwrap();
    // publish messages
    let num_of_message = 1000;
    publish_test_messages(&channel, exchange_name, routing_key, num_of_message).await;

    // cancel consumer
    channel
        .basic_cancel(BasicCancelArguments::new(&consumer_tag))
        .await
        .unwrap();

    // publish again
    publish_test_messages(&channel, exchange_name, routing_key, num_of_message).await;

    // wait for certain time, and the messages should not be consumed.
    time::sleep(time::Duration::from_secs(1)).await;

    // verify not all messages are consumed
    let args = QueueDeclareArguments::new(&queue_name)
        .passive(true)
        .finish();
    let (_, message_count, consumer_count) =
        channel.queue_declare(args.clone()).await.unwrap().unwrap();
    // check messages remain in queue
    assert_ne!(0, message_count);
    // check no consumer in server
    assert_eq!(0, consumer_count);

    // explicitly close
    channel.close().await.unwrap();
    connection.close().await.unwrap();
}

async fn publish_test_messages(
    channel: &Channel,
    exchange_name: &str,
    routing_key: &str,
    num: usize,
) {
    // contents to publish
    let content = String::from(
        r#"
            {
                "data": "publish data for consumer test" 
            }
        "#,
    )
    .into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, routing_key);

    // applicaton's headers
    let mut headers = FieldTable::new();
    headers.insert("date".try_into().unwrap(), "2022-11".into());

    let basic_props = BasicProperties::default()
        .with_content_type("application/json")
        .with_headers(headers)
        .with_delivery_mode(DELIVERY_MODE_TRANSIENT)
        .with_user_id("user")
        .with_app_id("consumer_test")
        .finish();

    let mut large_size_cnt = 0;
    let mut zero_size_cnt = 0;
    if num >= 3 {
        large_size_cnt = 1;
        zero_size_cnt = 1;
    }
    let regular_cnt = num - large_size_cnt - zero_size_cnt;
    for _ in 0..regular_cnt {
        channel
            .basic_publish(basic_props.clone(), content.clone(), args.clone())
            .await
            .unwrap();
    }
    // publish messages of zero body size
    for _ in 0..zero_size_cnt {
        channel
            .basic_publish(basic_props.clone(), Vec::new(), args.clone())
            .await
            .unwrap();
    }
    // publish messages of large body size
    for _ in 0..large_size_cnt {
        channel
            .basic_publish(basic_props.clone(), vec![1; 131082], args.clone())
            .await
            .unwrap();
    }
}
