use amqprs::{
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicNackArguments, BasicPublishArguments,
        BasicRejectArguments, Channel, QueueBindArguments, QueueDeclareArguments,
    },
    connection::Connection,
    consumer::{AsyncConsumer, BlockingConsumer},
    BasicProperties, Deliver,
};
use async_trait::async_trait;
use tokio::time;
mod common;

struct MixedConsumer {
    auto_ack: bool,
}

impl MixedConsumer {
    pub fn new(auto_ack: bool) -> Self {
        Self { auto_ack }
    }
}

#[async_trait]
impl AsyncConsumer for MixedConsumer {
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {
        tracing::info!(
            "receive message properties  {} on channel {}",
            basic_properties,
            channel
        );
        // ack explicitly if manual ack
        if !self.auto_ack {
            let tag = deliver.delivery_tag();
            let prio = basic_properties.priority().unwrap_or_else(|| 0);
            if prio <= 1 {
                tracing::info!("ack async");

                channel
                    .basic_ack(BasicAckArguments::new(tag, false))
                    .await
                    .unwrap();
            }
            // test nack
            else if prio == 2 {
                tracing::info!("nack async");

                channel
                    .basic_nack(BasicNackArguments::new(tag, false, false))
                    .await
                    .unwrap();
            }
            // test reject
            else {
                tracing::info!("reject async");

                channel
                    .basic_reject(BasicRejectArguments::new(tag, false))
                    .await
                    .unwrap();
            }
        }
    }
}

impl BlockingConsumer for MixedConsumer {
    fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        _content: Vec<u8>,
    ) {
        tracing::info!(
            "receive message properties {} on channel {}",
            basic_properties,
            channel
        );

        // ack explicitly if manual ack
        if !self.auto_ack {
            let tag = deliver.delivery_tag();
            let prio = basic_properties.priority().unwrap_or_else(|| 0);
            if prio <= 1 {
                tracing::info!("ack blocking");

                channel
                    .basic_ack_blocking(BasicAckArguments::new(tag, false))
                    .unwrap();
            }
            // test nack
            else if prio == 2 {
                tracing::info!("nack blocking");

                channel
                    .basic_nack_blocking(BasicNackArguments::new(tag, false, false))
                    .unwrap();
            }
            // test reject
            else {
                tracing::info!("reject blocking");

                channel
                    .basic_reject_blocking(BasicRejectArguments::new(tag, false))
                    .unwrap();
            }
        }
    }
}

#[tokio::test]
async fn test_mixed_type_consumer() {
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
    let routing_key = "mixed_consumer_type";
    consumer_channel
        .queue_bind(QueueBindArguments::new(
            &queue_name,
            exchange_name,
            routing_key,
        ))
        .await
        .unwrap();

    // start consumer with given name
    let args = BasicConsumeArguments::new(&queue_name, "async_consumer");

    consumer_channel
        .basic_consume(MixedConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // start consumer with generated name by server
    let args = BasicConsumeArguments::new(&queue_name, "blocking_consumer");
    consumer_channel
        .basic_consume_blocking(MixedConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    // open a channel dedicated for publisher on the connection
    let pub_channel = connection.open_channel(None).await.unwrap();
    // publish test messages
    publish_test_messages(&pub_channel, exchange_name, routing_key, 3).await;

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(1)).await;

    // explicitly close
    pub_channel.close().await.unwrap();
    consumer_channel.close().await.unwrap();
    connection.close().await.unwrap();
}

async fn publish_test_messages(
    channel: &Channel,
    exchange_name: &str,
    routing_key: &str,
    num: usize,
) {
    // contents to publish
    let content = String::from("testing mixed type consumer").into_bytes();

    // create arguments for basic_publish
    let args = BasicPublishArguments::new(exchange_name, routing_key);

    let mut basic_props = BasicProperties::default();

    for _ in 0..num {
        // expect ack from consumer
        basic_props.with_priority(1);
        channel
            .basic_publish(basic_props.clone(), content.clone(), args.clone())
            .await
            .unwrap();

        // expect nack from consumer
        basic_props.with_priority(2);
        channel
            .basic_publish(basic_props.clone(), content.clone(), args.clone())
            .await
            .unwrap();

        // expect reject from consumer
        basic_props.with_priority(3);
        basic_props.with_priority(3);
        channel
            .basic_publish(basic_props.clone(), content.clone(), args.clone())
            .await
            .unwrap();
    }
}
