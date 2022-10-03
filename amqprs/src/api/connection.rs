use amqp_serde::types::AmqpChannelId;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::frame::{Close, Declare, Frame, Open, OpenChannel, ProtocolHeader, StartOk, TuneOk};
use crate::net::{ConnectionManager, Message, SplitConnection};

use super::channel::Channel;
use super::error::Error;

pub struct Connection {
    manager: ConnectionManager,
}

impl Connection {
    pub async fn open(uri: &str) -> Result<Self, Error> {
        let mut connection = SplitConnection::open(uri).await?;
        // TODO: protocol header negotiation ?
        connection.write(&ProtocolHeader::default()).await?;

        // S: 'Start'
        let (_, start) = connection.read_frame().await?;

        // C: 'StartOk'
        let start_ok = StartOk::default().into_frame();
        connection.write_frame(0, start_ok).await?;

        // S: 'Tune'
        let (_, tune) = connection.read_frame().await?;
        println!("{tune:?}");

        // C: TuneOk
        let mut tune_ok = TuneOk::default();
        match tune {
            Frame::Tune(_, method) => {
                tune_ok.channel_max = method.channel_max;
                tune_ok.frame_max = method.frame_max;
                tune_ok.heartbeat = method.heartbeat;
            }
            _ => return Err(Error::ConnectionOpenFailure),
        };
        let channel_max = tune_ok.channel_max;
        let heartbeat = tune_ok.channel_max;
        connection.write_frame(0, tune_ok.into_frame()).await?;

        // C: Open
        let open = Open::default().into_frame();
        connection.write_frame(0, open).await?;

        // S: OpenOk
        let (_, open_ok) = connection.read_frame().await?;
        println!("{open_ok:?}");

        let manager = ConnectionManager::spawn(connection, channel_max).await;
        Ok(Self { manager })
    }

    pub async fn close(mut self) -> Result<(), Error> {
        // C: Close
        self.manager
            .tx
            .send((0, Close::default().into_frame()))
            .await?;

        // S: CloseOk
        let close_ok = self.manager.rx.recv().await;
        println!("{close_ok:?}");
        Ok(())
    }

    pub async fn channel(&mut self) -> Result<Channel, Error> {
        let (channel_id, tx, mut rx) = self.manager.allocate_channel().await;

        tx.send((channel_id, OpenChannel::default().into_frame()))
            .await?;
        match rx.recv().await {
            Some(frame) => match frame {
                Frame::OpenChannelOk(_, _) => Ok(Channel::new(channel_id, tx, rx)),
                _ => Err(Error::ChannelOpenFailure),
            },
            None => Err(Error::ChannelOpenFailure),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;
    use tokio::time;
    #[tokio::test]
    async fn test_api_open_and_close() {
        let mut client = Connection::open("localhost:5672").await.unwrap();

        let mut channel = client.channel().await.unwrap();
        channel.exchange_declare().await.unwrap();
        // time::sleep(time::Duration::from_secs(160)).await;
        channel.close().await;
        client.close().await.unwrap();
    }
}
