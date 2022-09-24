use std::io::Cursor;

use crate::frame::{Frame, FrameHeader, ProtocolHeader};

use amqp_serde::{constants::FRAME_END, from_bytes, into_buf, to_bytes, types::LongUint};
use bytes::{Buf, BufMut, BytesMut};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const DEFAULT_BUFFER_SIZE: usize = 8192;

pub struct Connection {
    stream: TcpStream,
    write_buffer: BytesMut,
    read_buffer: BytesMut,
}

impl Connection {
    pub async fn open(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let write_buffer = BytesMut::with_capacity(DEFAULT_BUFFER_SIZE);
        let read_buffer = BytesMut::with_capacity(DEFAULT_BUFFER_SIZE);

        let mut me = Self {
            stream,
            write_buffer,
            read_buffer,
        };

        //
        Ok(me)
    }
    pub async fn close(&mut self) -> io::Result<()> {
        self.stream.shutdown().await
    }

    pub async fn write<T: Serialize>(&mut self, value: &T) -> io::Result<usize> {
        into_buf(value, &mut self.write_buffer)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        let len = self.write_buffer.len();
        self.stream.write_all(&self.write_buffer).await?;
        self.write_buffer.advance(len);
        Ok(len)
    }

    pub async fn write_frame<T: Serialize>(&mut self, mut frame: Frame<T>) -> io::Result<usize> {
        into_buf(&frame.payload, &mut self.write_buffer).unwrap();
        // update payload size
        let payload_size = self.write_buffer.len();
        frame.set_payload_size(payload_size as LongUint);

        into_buf(&frame.header, &mut self.write_buffer).unwrap();
        self.stream
            .write_all(self.write_buffer.get(payload_size..).unwrap())
            .await?;
        self.stream
            .write_all(self.write_buffer.get(..payload_size).unwrap())
            .await?;
        self.stream.write_u8(FRAME_END).await?;
        let len = self.write_buffer.len();
        self.write_buffer.advance(len);
        // total length + frame end byte
        Ok(len + 1)

        ////////////////////////////////////////////////////////////////
        // into_buf(&frame, &mut self.write_buffer).unwrap();
        // let len = self.write_buffer.len()
        // let mut start = 3;
        // for b in (len as LongUint - 8).to_be_bytes() {
        //     let p = self.write_buffer.get_mut(start).unwrap();
        //     *p = b;
        //     start += 1;
        // }
        // self.stream.write_all(&self.write_buffer).await?;
        // self.write_buffer.advance(len);
        // Ok(len)

        ////////////////////////////////////////////////////////////////
        // let payload = to_bytes(&frame.payload)
        //     .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        // // update the payload size
        // frame.set_payload_size(payload.len() as LongUint);
        // let header = to_bytes(&frame.header)
        //     .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        // self.stream.write_all(&header).await?;
        // self.stream.write_all(&payload).await?;
        // self.stream.write_u8(FRAME_END).await?;
        // Ok(header.len() + payload.len() + 1)
    }

    pub async fn read_frame<T: DeserializeOwned>(&mut self) -> io::Result<Frame<T>> {
        // TODO: handle network error, such as
        // 1. timeout ?
        // 2.  incomplete frame?
        loop {
            let len = self.stream.read_buf(&mut self.read_buffer).await?;
            if len == 0 {
                if self.read_buffer.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::Other, "peer shutdown"));
                } else {
                    return Err(io::Error::new(io::ErrorKind::Other, "connection failure"));
                }
            }
            println!("number of bytes read from network {len}");
            // println!("{:02X?}", self.read_buffer.as_ref());
            // println!("{:?}", self.read_buffer);

            // check frame type and properties, 7 octects
            if self.read_buffer.len() > 7 {
                let header: FrameHeader = from_bytes(self.read_buffer.get(0..7).unwrap()).unwrap();
                // have full frame
                let total = header.payload_size as usize + 8;
                if total == self.read_buffer.len() {
                    // check frame end
                    let last_byte = self.read_buffer.last().unwrap();
                    match last_byte {
                        &FRAME_END => {
                            let frame: Frame<T> = from_bytes(self.read_buffer.as_ref()).unwrap();
                            self.read_buffer.advance(total);
                            return Ok(frame);
                        }
                        _ => panic!("not a valid frame"),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use amqp_serde::{
        constants::FRAME_END,
        to_bytes,
        types::{LongStr, PeerProperties, ShortStr},
    };
    use tokio::runtime;

    use crate::frame::{Frame, FrameHeader, Start, StartOk, Tune, ProtocolHeader, TuneOk, Open, OpenOk};

    use super::Connection;

    fn new_runtime() -> runtime::Runtime {
        let rt = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt
    }

    #[test]
    fn test_client_establish_connection() {
        // connection       = open-connection *use-connection close-connection
        // open-connection  = C:protocolheader
        //                 S:START C:STARTOK
        //                 *challenge
        //                 S:TUNE C:TUNEOK
        //                 C:OPEN S:OPENOK
        // challenge        = S:SECURE C:SECUREOK
        // use-connection   = *channel
        // close-connection = C:CLOSE S:CLOSEOK
        //                 / S:CLOSE C:CLOSEOK
        let rt = new_runtime();
        rt.block_on(async {
            let mut conn = Connection::open("localhost:5672").await.unwrap();

            // C: protocol-header
            conn.write(&ProtocolHeader::default()).await.unwrap();

            // S: 'Start'
            let start = conn.read_frame::<Start>().await.unwrap();
            println!("{start:?}");

            // C: 'StartOk'
            let start_ok = Frame::new_method(StartOk::default());
            conn.write_frame(start_ok).await.unwrap();

            // S: 'Tune'
            let tune = conn.read_frame::<Tune>().await.unwrap();
            println!("{tune:?}");

            // C: TuneOk
            let mut tune_ok = Frame::new_method(TuneOk::default());
            tune_ok.payload.channel_max = tune.payload.channel_max;
            tune_ok.payload.frame_max = tune.payload.frame_max;
            tune_ok.payload.heartbeat = tune.payload.heartbeat;
            conn.write_frame(tune_ok).await.unwrap();

            // C: Open
            let open = Frame::new_method(Open::default());
            conn.write_frame(open).await.unwrap();

            // S: OpenOk
            let open_ok = conn.read_frame::<OpenOk>().await.unwrap();
            println!("{open_ok:?}");
        });
    }
}
