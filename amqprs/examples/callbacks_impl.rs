//! Template for callbacks implementation.
//!
use amqprs::{
    callbacks::{ChannelCallback, ConnectionCallback},
    channel::Channel,
    connection::{Connection, OpenConnectionArguments},
    Ack, BasicProperties, Cancel, Close, CloseChannel, Nack, Return,
};
use async_trait::async_trait;

////////////////////////////////////////////////////////////////////////////////
type Result<T> = std::result::Result<T, amqprs::error::Error>;

////////////////////////////////////////////////////////////////////////////////
struct ExampleConnectionCallback;

#[allow(unused_variables,  /* template */)]
#[async_trait]
impl ConnectionCallback for ExampleConnectionCallback {
    async fn close(&mut self, connection: &Connection, close: Close) -> Result<()> {
        Ok(())
    }

    async fn blocked(&mut self, connection: &Connection, reason: String) {}
    async fn unblocked(&mut self, connection: &Connection) {}
}

////////////////////////////////////////////////////////////////////////////////
struct ExampleChannelCallback;

#[allow(unused_variables, /* template */)]
#[async_trait]
impl ChannelCallback for ExampleChannelCallback {
    async fn close(&mut self, channel: &Channel, close: CloseChannel) -> Result<()> {
        Ok(())
    }
    async fn cancel(&mut self, channel: &Channel, cancel: Cancel) -> Result<()> {
        Ok(())
    }
    async fn flow(&mut self, channel: &Channel, active: bool) -> Result<bool> {
        Ok(true)
    }
    async fn publish_ack(&mut self, channel: &Channel, ack: Ack) {}
    async fn publish_nack(&mut self, channel: &Channel, nack: Nack) {}
    async fn publish_return(
        &mut self,
        channel: &Channel,
        ret: Return,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
    }
}

////////////////////////////////////////////////////////////////////////////////
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // open a connection to RabbitMQ server
    let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

    let connection = Connection::open(&args).await.unwrap();
    connection
        .register_callback(ExampleConnectionCallback)
        .await
        .unwrap();

    // open a channel on the connection
    let channel = connection.open_channel(None).await.unwrap();
    channel
        .register_callback(ExampleChannelCallback)
        .await
        .unwrap();
}
