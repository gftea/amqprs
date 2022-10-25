use amqprs::api::{channel::ExchangeDeclareArguments, connection::Connection};

#[tokio::test]
async fn test_publish() {
    let mut client = Connection::open("localhost:5672").await.unwrap();

    let mut channel = client.open_channel().await.unwrap();
    let mut args = ExchangeDeclareArguments::new("amq.direct", "direct");
    args.passive = true;
    // usage 1: just create default arguments and update
    channel.exchange_declare(args).await.unwrap();
    // usage 2: move partial default value  to new struct and update
    channel
        .exchange_declare(ExchangeDeclareArguments {
            passive: true,
            ..ExchangeDeclareArguments::new("amq.direct", "direct")
        })
        .await
        .unwrap();
}
