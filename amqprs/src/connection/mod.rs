use std::io::Cursor;

use crate::frame::{Frame, FrameHeader, ProtocolHeader};

use amqp_serde::{constants::FRAME_END, from_bytes, to_bytes, types::LongUint};
use bytes::{Buf, BytesMut};
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

    pub async fn negotiate_protocol(&mut self) -> io::Result<usize> {
        let content = to_bytes(&ProtocolHeader::default())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        self.stream.write_all(content.as_ref()).await?;
        // TODO: if server reject the protocol, try to negotiate!

        Ok(content.len())
    }

    pub async fn write_frame<T: Serialize>(&mut self, mut frame: Frame<T>) -> io::Result<usize> {
        // TODO: add new api to serialize the frame into write_buffer instead

        let payload = to_bytes(&frame.payload)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        // update the payload size
        frame.set_payload_size(payload.len() as LongUint);        
        let header = to_bytes(&frame.header)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        // TODO: cannot get the bytes size in advance
        // so we serialzie the frame first, then update the size in serialized bytes sequence
        // let payload_size = content.len() as LongUint - 8;
        // println!("payload size = {}", payload_size);
        // let mut start = 3;
        // for b in payload_size.to_be_bytes() {
        //     let p = content.get_mut(start).unwrap();
        //     *p = b;
        //     start += 1;
        // }
        self.stream.write_all(&header).await?;
        self.stream.write_all(&payload).await?;
        self.stream.write_u8(FRAME_END).await?;
        Ok(header.len() + payload.len() + 1)
    }

    pub async fn read_frame<T: DeserializeOwned>(&mut self) -> io::Result<Frame<T>> {
        // TODO: read frame type, channel, payload size
        // processing according to frame type
        // if method frame, process according to class-id and method-id
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

    use crate::frame::{Frame, FrameHeader, Start, StartOk, Tune};

    use super::Connection;

    fn new_runtime() -> runtime::Runtime {
        let rt = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt
    }

    #[test]
    fn test_open() {
        let rt = new_runtime();
        rt.block_on(async {
            let mut conn = Connection::open("localhost:5672").await.unwrap();

            // C: protocol-header
            conn.negotiate_protocol().await.unwrap();

            // S: 'Start'
            let start = conn.read_frame::<Start>().await.unwrap();
            println!("{start:?}");

            // C: 'StartOk'
            let start_ok = Frame::new_method_frame(StartOk::default());
            conn.write_frame(start_ok).await.unwrap();

            // S: 'Tune'
            let secure = conn.read_frame::<Tune>().await.unwrap();
            println!("{secure:?}");
        });
    }
}
