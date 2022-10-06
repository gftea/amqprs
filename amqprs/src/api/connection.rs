use crate::frame::{
    Close, Frame, Open, OpenChannel, ProtocolHeader, StartOk, TuneOk, CTRL_CHANNEL,
};
use crate::net::{ConnectionManager, SplitConnection};

use super::channel::Channel;
use super::error::Error;

pub struct Connection {
    uri: String,
    manager: ConnectionManager,
}

/// AMQP Connection API
///
impl Connection {
    /// Open a AMQP connection
    pub async fn open(uri: &str) -> Result<Self, Error> {
        // TODO: uri parsing
        let mut connection = SplitConnection::open(uri).await?;

        // TODO: protocol header negotiation ?
        connection.write(&ProtocolHeader::default()).await?;

        // S: 'Start'
        let (_, frame) = connection.read_frame().await?;
        get_expected_method!(
            frame,
            Frame::Start,
            Error::ConnectionOpenError("start".to_string())
        )?;

        // C: 'StartOk'
        let start_ok = StartOk::default().into_frame();
        connection.write_frame(CTRL_CHANNEL, start_ok).await?;

        // S: 'Tune'
        let (_, frame) = connection.read_frame().await?;
        let tune = get_expected_method!(
            frame,
            Frame::Tune,
            Error::ConnectionOpenError("tune".to_string())
        )?;
        // C: TuneOk
        let mut tune_ok = TuneOk::default();
        tune_ok.channel_max = tune.channel_max;
        tune_ok.frame_max = tune.frame_max;
        tune_ok.heartbeat = tune.heartbeat;

        let channel_max = tune_ok.channel_max;
        let _heartbeat = tune_ok.channel_max;
        connection
            .write_frame(CTRL_CHANNEL, tune_ok.into_frame())
            .await?;

        // C: Open
        let open = Open::default().into_frame();
        connection.write_frame(CTRL_CHANNEL, open).await?;

        // S: OpenOk
        let (_, frame) = connection.read_frame().await?;
        get_expected_method!(frame, Frame::OpenOk, Error::ChannelOpenError(CTRL_CHANNEL.to_string()))?;

        let manager = ConnectionManager::spawn(connection, channel_max).await;
        Ok(Self {
            uri: uri.to_string(),
            manager,
        })
    }

    pub async fn close(mut self) -> Result<(), Error> {
        synchronous_request!(
            self.manager,
            (CTRL_CHANNEL, Close::default().into_frame()),
            self.manager,
            Frame::CloseOk,
            (),
            Error::ConnectionCloseError(CTRL_CHANNEL.to_string())
        )
    }

    pub async fn channel(&mut self) -> Result<Channel, Error> {
        let (channel_id, tx, mut rx) = self.manager.allocate_channel().await;

        synchronous_request!(
            tx,
            (channel_id, OpenChannel::default().into_frame()),
            rx,
            Frame::OpenChannelOk,
            Channel::new(channel_id, tx, rx),
            Error::ChannelOpenError(channel_id.to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;
    use tokio::time;

    #[tokio::test]
    async fn test_channel_open_use_close() {
        let mut client = Connection::open("localhost:5672").await.unwrap();

        let mut channel = client.channel().await.unwrap();
        channel.exchange_declare().await.unwrap();
        // time::sleep(time::Duration::from_secs(160)).await;
        channel.close().await.unwrap();
        client.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_multi_channel_open_close() {
        let mut client = Connection::open("localhost:5672").await.unwrap();

        let mut handles = vec![];

        for _ in 0..10 {
            let mut ch = client.channel().await.unwrap();
            handles.push(tokio::spawn(async move {
                time::sleep(time::Duration::from_secs(1)).await;
                ch.exchange_declare().await.unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_multi_conn_open_close() {
        let mut handles = vec![];
        for i in 0..10 {
            let handle = tokio::spawn(async move {
                let client = Connection::open("localhost:5672").await.unwrap();
                time::sleep(time::Duration::from_millis((i % 3) * 50 + 100)).await;
                client.close().await.unwrap();
            });
            handles.push(handle);
        }
        for h in handles {
            h.await.unwrap();
        }
    }
}
