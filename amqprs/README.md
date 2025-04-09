[![ci](https://github.com/gftea/amqprs/actions/workflows/regression_test.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/regression_test.yml)
[![codecov](https://codecov.io/gh/gftea/amqprs/branch/main/graph/badge.svg?token=7MF92R6F60)](https://codecov.io/gh/gftea/amqprs)
[![Documentation](https://docs.rs/amqprs/badge.svg)](https://docs.rs/amqprs)
[![crates.io](https://img.shields.io/crates/v/amqprs.svg)](https://crates.io/crates/amqprs)
[![Discord](https://img.shields.io/discord/1065607081513717900)](https://discord.gg/g7Z9TeCu28)

# MSRV

- Since `v2.0.0`, it has breaking changes due to rustls upgrade, and the msrv is `1.71`.
- Version < `v2.0.0`: the msrv is `1.56`

## NOTE! Please upgrade to v2 version, because no bug fix or enhancement will be applied to v1 version!

# What is "amqprs"

Yet another RabbitMQ client implementation in rust with different design goals.

The library is accepted to list in [RabbitMQ official website](https://www.rabbitmq.com/devtools.html#rust-dev).

It's probably the best performance among existing Rust clients. See 
[Benchmarks](https://github.com/gftea/amqprs/blob/main/benchmarks/README.md).

## Design Philosophy

1. API first: easy to use and understand. Keep the API similar as python client library so that it is easier for users to move from there.
2. Minimum external dependencies: as few external crates as possible.
3. lock free: no mutex/lock in client library itself.

# Design Architecture
![Lock-free Design](https://github.com/gftea/amqprs/raw/HEAD/architecture.png)

# Quick Start: Consume and Publish

```rust
// open a connection to RabbitMQ server
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
let routing_key = "amqprs.example";
let exchange_name = "amq.topic";
channel
    .queue_bind(QueueBindArguments::new(
        &queue_name,
        exchange_name,
        routing_key,
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
let args = BasicPublishArguments::new(exchange_name, routing_key);

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

# Typical Examples

## [Example - Publish and Subscribe](https://github.com/gftea/amqprs/blob/main/examples/src/basic_pub_sub.rs)
## [Example - SSL/TLS](https://github.com/gftea/amqprs/blob/main/examples/src/tls.rs)

# Optional Features

- "traces": enable `tracing` in the library.
- "compliance_assert": enable compliance assertion according to AMQP spec.
  If enabled, library always check user inputs and `panic` if any non-compliance.
  If disabled, then it relies on server to reject.
- "tls": enable SSL/TLS.
- "urispec": enable support of [RabbitMQ URI Specification](https://www.rabbitmq.com/uri-spec.html)


# Run Test Locally

__Testing depends on RabbitMQ docker container.__

```bash
# start rabbitmq server
./start_rabbitmq.sh

# run tests
./regression_test.sh

# enable traces in test.
# Note that it only takes effect if "traces" feature is enabled
RUST_LOG=debug ./regression_test.sh
```

# Benchmarks

[See benchmarks's README](https://github.com/gftea/amqprs/blob/main/benchmarks/README.md)

<details>
<summary>Community Feedbacks</summary>

#### Luc Georges @ Hugging Face 

> I've put amqprs in production and it's working very nicely so far! I've had spikes of publish and delivery over 10k msg/sec without breaking a sweat
    
#### Michael Klishin @ RabbitMQ team
    
> We usually add new clients after they get some traction in the community. But this client seems to be fairly well documented and I like the API (it is a bit complicated with some other Rust clients)
    
</details>

