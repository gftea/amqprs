[![integration-test](https://github.com/gftea/amqprs/actions/workflows/rust.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/rust.yml)

# amqprs

Yet another RabbitMQ client implementation in rust with different design goals.

## Design philosophy

1. API first: easy to use, easy to understand. Keep the API similar as python client library so that it is easier for users to move from there.
2. Minimum external dependencies: as less exteranl crates as possible
3. lock free: no mutex/lock in client library itself 


## Example: Consume and Publish

[Example source code](amqprs/examples/basic_pub_sub.rs) 

```rust
// open a connection to RabbitMQ server
let args = OpenConnectionArguments::new("localhost:5672", "user", "bitnami");

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
let exchange_name = "amq.topic";
channel
    .queue_bind(QueueBindArguments::new(
        &queue_name,
        exchange_name,
        "eiffel.#",
    ))
    .await
    .unwrap();

//////////////////////////////////////////////////////////////////////////////
// start consumer with given name
channel
    .basic_consume(DefaultConsumer::new(args.no_ack), BasicConsumeArguments::new(&queue_name, "example_basic_pub_sub"))
    .await
    .unwrap();

//////////////////////////////////////////////////////////////////////////////
// publish a message
let content = String::from(
    r#"
        {
            "meta": {"id": "f9d42464-fceb-4282-be95-0cd98f4741b0", "type": "PublishTester", "version": "4.0.0", "time": 1640035100149},
            "data": { "customData": []}, 
            "links": [{"type": "BASE", "target": "fa321ff0-faa6-474e-aa1d-45edf8c99896"}]
        }
    "#).into_bytes();

channel
    .basic_publish(BasicProperties::default(), content, BasicPublishArguments::new(exchange_name, "eiffel.a.b.c.d"))
    .await
    .unwrap();

// keep the `channel` and `connection` object from dropping
// NOTE: channel/connection will be closed when drop
time::sleep(time::Duration::from_secs(10)).await;

```


## Design Architecture
![Lock-free Design](amqp-chosen_design.drawio.png) 