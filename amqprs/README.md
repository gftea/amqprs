[![ci](https://github.com/gftea/amqprs/actions/workflows/regression_test.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/regression_test.yml)
[![codecov](https://codecov.io/gh/gftea/amqprs/branch/main/graph/badge.svg?token=7MF92R6F60)](https://codecov.io/gh/gftea/amqprs)
[![Documentation](https://docs.rs/amqprs/badge.svg)](https://docs.rs/amqprs)
[![crates.io](https://img.shields.io/crates/v/amqprs.svg)](https://crates.io/crates/amqprs)

# amqprs

Yet another RabbitMQ client implementation in rust with different design goals.

## Design philosophy

1. API first: easy to use and understand. Keep the API similar as python client library so that it is easier for users to move from there.
2. Minimum external dependencies: as few external crates as possible.
3. lock free: no mutex/lock in client library itself.

# Design Architecture
![Lock-free Design](amqp-chosen_design.drawio.png)

# Example: Consume and Publish

## [Link to full example code](https://github.com/gftea/amqprs/blob/main/examples/src/basic_pub_sub.rs)

```rust
// open a connection to RabbitMQ server
let connection = Connection::open(&OpenConnectionArguments::new(
    "localhost:5672",
    "user",
    "bitnami",
))
.await
.unwrap();
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

//////////////////////////////////////////////////////////////////
// start consumer with given name
let args = BasicConsumeArguments::new(
        &queue_name,
        "example_basic_pub_sub"
    );

channel
    .basic_consume(DefaultConsumer::new(args.no_ack), args)
    .await
    .unwrap();

//////////////////////////////////////////////////////////////////
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


// channel/connection will be closed when drop.
// keep the `channel` and `connection` object from dropping
// before pub/sub is done.
time::sleep(time::Duration::from_secs(1)).await;
// explicitly close
channel.close().await.unwrap();
connection.close().await.unwrap();
```

# Optional Features

- "traces": enable `tracing` in the library.
- "compliance_assert": enable compliance assertion according to AMQP spec.
    If enabled, library always check user inputs and `panic` if any non-compliance.
    If disabled, then it relies on server to reject.
- "tls": enable SSL/TLS.
- "urispec": enable support of [RabbitMQ URI Specification](https://www.rabbitmq.com/uri-spec.html)
