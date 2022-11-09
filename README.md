[![integration-test](https://github.com/gftea/amqprs/actions/workflows/rust.yml/badge.svg)](https://github.com/gftea/amqprs/actions/workflows/rust.yml)

# amqprs

## Design goals
1. easy to use API
2. easy to understand API
3. lock free

keep the API similar as python client library so that easier for user to move from there.


## Example: Publish
```rust
    // open a connection to RabbitMQ server
    let connection = Connection::open("localhost:5672").await.unwrap();

    // open a channel on the connection
    let mut channel = connection.open_channel().await.unwrap();

    let exchange_name = "amq.topic";
    let exchange_type = "topic";

    // create arguments for exchange_declare
    let mut args = ExchangeDeclareArguments::new(exchange_name, exchange_type);
    // set to passive mode - checking existence of the exchange
    args.passive = true;
    // declare exchange 
    channel.exchange_declare(args).await.unwrap();

    // contents to publish 
    let content = String::from(
        r#"
            {
                "meta": {"id": "f9d42464-fceb-4282-be95-0cd98f4741b0", "type": "PublishTester", "version": "4.0.0", "time": 1640035100149},
                "data": { "customData": []}, 
                "links": [{"type": "BASE", "target": "fa321ff0-faa6-474e-aa1d-45edf8c99896"}]}
        "#
        ).into_bytes();
    
    // create arguments for basic_publish
    let mut args = BasicPublishArguments::new();
    // set target exchange name 
    args.exchange = exchange_name.to_string();
    // basic publish
    channel
        .basic_publish(args, BasicProperties::default(), content)
        .await
        .unwrap();
    
    // NOTE: channel/connection will be closed when drop
    // keep the `channel` and `connection` object from dropping until publish is done
    time::sleep(time::Duration::from_secs(1)).await;
```


## Example: Consume
```rust
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
    channel
        .queue_bind(QueueBindArguments::new(queue_name, "amq.topic", "eiffel.#"))
        .await
        .unwrap();

    // create arguments for consume
    let mut args = BasicConsumeArguments::new();
    args.queue = queue_name.to_string();
    args.consumer_tag = "amqprs-consumer-example".to_string();

    // start consumer to consume messages
    // User can define any type implements `Consumer` trait
    // `DefaultConsumer` is provided for testing only now.
    channel.basic_consume(args, DefaultConsumer).await.unwrap();

    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(60)).await;
```