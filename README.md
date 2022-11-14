[![integration-test](https://github.com/gftea/amqprs/actions/workflows/rust.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/rust.yml)

# amqprs

Yet another RabbitMQ client implementation in rust with different design goals.

## Design philosophy
1. API first: easy to use, easy to understand. Keep the API similar as python client library so that it is easier for users to move from there.
2. Minimum external dependencies: as less exteranl crates as possible
3. lock free: no mutex/lock in client library itself 


## Example: Consume and Publish

[See basic example](amqprs/examples/basic_pub_sub.rs) 

```rust

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    // open a channel on the connection
    let mut channel = connection.open_channel().await.unwrap();

    // declare a queue
    let queue_name = "amqprs";
    channel
        .queue_declare(QueueDeclareArguments::new(queue_name))
        .await
        .unwrap();

    // bind the queue to exchange
    let exchange_name = "amq.topic";
    channel
        .queue_bind(QueueBindArguments::new(
            queue_name,
            exchange_name,
            "eiffel.#",
        ))
        .await
        .unwrap();

    //////////////////////////////////////////////////////////////////////////////
    // start consumer with given name
    let mut args = BasicConsumeArguments::new();
    args.queue = queue_name.to_string();
    args.consumer_tag = "amqprs-consumer-example".to_string();

    channel
        .basic_consume(DefaultConsumer::new(args.no_ack), args)
        .await
        .unwrap();

    //////////////////////////////////////////////////////////////////////////////
    // publish message
    let content = String::from(
        r#"
            {
                "meta": {"id": "f9d42464-fceb-4282-be95-0cd98f4741b0", "type": "PublishTester", "version": "4.0.0", "time": 1640035100149},
                "data": { "customData": []}, 
                "links": [{"type": "BASE", "target": "fa321ff0-faa6-474e-aa1d-45edf8c99896"}]
            }
        "#
        ).into_bytes();

    // create arguments for basic_publish
    let mut args = BasicPublishArguments::new();
    // set target exchange name
    args.exchange = exchange_name.to_string();
    args.routing_key = "eiffel.a.b.c.d".to_string();

    channel
        .basic_publish(BasicProperties::default(), content, args)
        .await
        .unwrap();

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(10)).await;
}

```

## Design Architecture
![Lock-free Design](amqp-chosen_design.drawio.png) 