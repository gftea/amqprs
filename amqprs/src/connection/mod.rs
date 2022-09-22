use std::io::Cursor;

use crate::frame::{Frame, ProtocolHeader};

use amqp_serde::{constants::FRAME_END, from_bytes, to_bytes};
use bytes::{Buf, BytesMut};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
pub struct Connection {
    stream: TcpStream,
    write_buffer: BytesMut,
    read_buffer: BytesMut,
}

impl Connection {
    pub async fn open(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let write_buffer = BytesMut::with_capacity(1024);
        let read_buffer = BytesMut::with_capacity(1024);

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

    pub async fn write_frame<T: Serialize>(&mut self, frame: Frame<T>) -> io::Result<usize> {
        // add new api to serialize the frame into write_buffer instead
        // instead of pass in whole Frame<T>, pass the frame payload type, because we need to calculate the payload size
        let content = to_bytes(&frame)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        self.stream.write_all(content.as_ref()).await?;

        Ok(content.len())
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
                #[derive(Deserialize)]
                struct Header {
                    typ: u8,
                    channel: u16,
                    size: u32,
                };
                let header: Header = from_bytes(self.read_buffer.get(0..7).unwrap()).unwrap();
                // have full frame
                let total = header.size as usize + 8;
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

    use crate::frame::{Frame, MethodPayload, Start, StartOk, Tune};

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
            conn.negotiate_protocol().await.unwrap();
            let start = conn.read_frame::<MethodPayload<Start>>().await.unwrap();
            println!("{start:?}");

            let payload = MethodPayload {
                class_id: 10,
                method_id: 11,
                method: StartOk {
                    client_properties: PeerProperties::new(),
                    machanisms: ShortStr::from("PLAIN"),
                    response: LongStr::from("\0user\0bitnami"),
                    locale: ShortStr::from("en_US"),
                },
            };
            // TODO: frame propertities is encoded wihtout using serialize API
            let payload_size = to_bytes(&payload).unwrap().len() as u32;
            let start_ok = Frame {
                typ: 1,
                channel: 0,
                payload_size,
                payload,
                frame_end: FRAME_END,
            };
            conn.write_frame(start_ok).await.unwrap();
            let secure = conn.read_frame::<MethodPayload<Tune>>().await.unwrap();
            println!("{secure:?}");
        });
    }
}
