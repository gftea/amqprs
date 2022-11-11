use amqprs::api::{
    channel::{BasicConsumeArguments, QueueBindArguments, QueueDeclareArguments},
    connection::Connection,
    consumer::DefaultConsumer,
};
use tokio::time;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_consume() {
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

    // start consumer with given name
    let mut args = BasicConsumeArguments::new();
    args.queue = queue_name.to_string();
    args.consumer_tag = "amqprs-consumer-example".to_string();

    channel.basic_consume(args, DefaultConsumer).await.unwrap();

    // start consumer with generated name by server
    let mut args = BasicConsumeArguments::new();
    args.queue = queue_name.to_string();
    channel.basic_consume(args, DefaultConsumer).await.unwrap();


    // recover unacknowledged messages
    // Edge case: 
    //  When basic_consume start, we immediately got unacknowledge messages redelivered from servers
    //  before ACK for a message with delivery_tag = 'A' is received by server, meanwhile server receive basic_recover, it will 
    //  redeliver the message with different delivery_tag = 'B' and forget about tag 'A',
    //  then once the ACK with delivery_tag = 'A' is received by server later, server will report channel exception.
    // 
    // Workaround: wait for all redelivered messages are handled
    time::sleep(time::Duration::from_secs(1)).await;

    channel.basic_recover(true).await.unwrap();


    // keep the `channel` and `connection` object from dropping
    // NOTE: channel/connection will be closed when drop
    time::sleep(time::Duration::from_secs(10)).await;

    // TODO: move to separate test case, below is for test only
    if true {
        // implicitly close by drop
        drop(channel);
        drop(connection);
    } else {
        // explicitly close
        channel.close().await.unwrap();
        connection.close().await.unwrap();
    }
    time::sleep(time::Duration::from_secs(1)).await;

}
