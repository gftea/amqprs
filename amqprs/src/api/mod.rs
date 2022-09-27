use crate::frame::*;
use crate::net::Connection as RawConnection;

pub struct Connection {
    net_conn: RawConnection,
    // channels: HashMap<usize, Sender<Frame<T>>>,
}
pub struct Channel {
    id: usize,
}

impl Connection {
    pub async fn open(uri: &str) -> Self {
        let mut conn = RawConnection::open(uri).await.unwrap();

        // C: protocol-header
        conn.write(&ProtocolHeader::default()).await.unwrap();

        // S: 'Start'
        let start = conn.read_frame().await.unwrap();
        println!(" {start:?}");

        // C: 'StartOk'
        let start_ok = StartOk::default().into_frame();
        conn.write_frame(0, start_ok).await.unwrap();

        // S: 'Tune'
        let tune = conn.read_frame().await.unwrap();
        println!("{tune:?}");

        // C: TuneOk
        let mut tune_ok = TuneOk::default();
        let tune = match tune.1 {
            Frame::Tune(_, v) => v,
            _ => panic!("wrong message"),
        };

        tune_ok.channel_max = tune.channel_max;
        tune_ok.frame_max = tune.frame_max;
        tune_ok.heartbeat = tune.heartbeat;

        conn.write_frame(0, tune_ok.into_frame()).await.unwrap();

        // C: Open
        let open = Open::default().into_frame();
        conn.write_frame(0, open).await.unwrap();

        // S: OpenOk
        let open_ok = conn.read_frame().await.unwrap();
        println!("{open_ok:?}");

        Connection { net_conn: conn }
    }

    pub async fn close(&mut self) {
        // C: Close
        self.net_conn
            .write_frame(0, Close::default().into_frame())
            .await
            .unwrap();

        // S: CloseOk
        let close_ok = self.net_conn.read_frame().await.unwrap();
        println!("{close_ok:?}");
    }

    pub async fn channel(&mut self, id: usize) -> Channel {
        Channel { id }
    }
}

#[cfg(test)]
mod tests {
    use super::Connection;

    #[tokio::test]
    async fn test_api_open_and_close() {
        let mut conn = Connection::open("localhost:5672").await;
        conn.close().await;
    }
}
