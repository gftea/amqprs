use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use amqp_serde::types::ShortUint;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::RwLock;

use crate::frame::{Frame, Open, ProtocolHeader, StartOk, TuneOk, Close};
use crate::net::{Reader, SplitConnection, Writer};

use super::error::Error;

#[derive(Debug)]
pub struct Message {
    channel_id: ShortUint,
    frame: Frame,
}
pub struct ReadConnection {
    reader: Reader,
    channels: Arc<RwLock<BTreeMap<ShortUint, Sender<Message>>>>,
}
pub struct WriteConnection {
    writer: Writer,
    rx: Receiver<Message>,
}
const CHANNEL_BUFFER_SIZE: usize = 16;
pub struct Connection {
    channel_id: ShortUint,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    channels: Arc<RwLock<BTreeMap<ShortUint, Sender<Message>>>>,
}
pub struct Channel {
    id: usize,
}

impl Connection {
    pub async fn open(uri: &str) -> Result<Self, Error> {
        let (tx_resp, mut rx_resp) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (tx_req, mut rx_req) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        let (mut reader, mut writer) = SplitConnection::open(uri).await.unwrap();
        // TODO: protocol header negotiation in connection level
        // do not need support channel multiplex, only done once per new connection
        writer.write(&ProtocolHeader::default()).await.unwrap();

        // create task for write connection
        tokio::spawn(async move {
            let mut conn = WriteConnection { writer, rx: rx_req };
            while let Some(msg) = conn.rx.recv().await {
                let Message { channel_id, frame } = msg;
                conn.writer.write_frame(channel_id, frame).await.unwrap();
            }
        });
        // store the channel's sender half
        let mut channels = Arc::new(RwLock::new(BTreeMap::new()));
        channels.write().await.insert(0, tx_resp);

        // create task for read connection
        let mut conn = ReadConnection {
            reader,
            channels: channels.clone(),
        };
        // response from channel over reader half
        tokio::spawn(async move {
            while let Ok((channel_id, frame)) = conn.reader.read_frame().await {
                let channels = conn.channels.read().await;

                let tx_resp = channels.get(&channel_id).unwrap();
                tx_resp.send(Message { channel_id, frame }).await.unwrap();
            }
        });

        // connection class method always use channel 0
        let channel_id = 0;
        // S: 'Start'
        let start = rx_resp.recv().await.unwrap();
        println!(" {start:?}");

        // C: 'StartOk'
        let start_ok = StartOk::default().into_frame();
        tx_req
            .send(Message {
                channel_id,
                frame: start_ok,
            })
            .await
            .unwrap();

        // S: 'Tune'
        let tune = rx_resp.recv().await.unwrap();
        println!("{tune:?}");

        // C: TuneOk
        let mut tune_ok = TuneOk::default();
        let tune = match tune.frame {
            Frame::Tune(_, v) => v,
            _ => panic!("wrong message"),
        };

        tune_ok.channel_max = tune.channel_max;
        tune_ok.frame_max = tune.frame_max;
        tune_ok.heartbeat = tune.heartbeat;

        tx_req
            .send(Message {
                channel_id,
                frame: tune_ok.into_frame(),
            })
            .await
            .unwrap();

        // C: Open
        let open = Open::default().into_frame();
        tx_req
            .send(Message {
                channel_id,
                frame: open,
            })
            .await
            .unwrap();

        // S: OpenOk
        let open_ok = rx_resp.recv().await.unwrap();
        println!("{open_ok:?}");

        Ok(Self {
            channel_id: 0,
            tx: tx_req,
            rx: rx_resp,
            channels,
        })
    }

    pub async fn close(&mut self) {
        // C: Close
        self.tx
            .send(Message {channel_id: self.channel_id, frame:Close::default().into_frame()})
            .await
            .unwrap();

        // S: CloseOk
        let close_ok = self.rx.recv().await.unwrap();
        println!("{close_ok:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;

    #[tokio::test]
    async fn test_api_open_and_close() {
        let mut conn = Connection::open("localhost:5672").await.unwrap();
        conn.close().await;
    }
}
