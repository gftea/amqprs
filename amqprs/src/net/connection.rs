use crate::frame::{Frame, FrameHeader};

use amqp_serde::{constants::FRAME_END, to_buffer, types::ShortUint};
use bytes::{Buf, BytesMut};
use serde::Serialize;
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

        Ok(Self {
            stream,
            write_buffer,
            read_buffer,
        })
    }
    pub async fn close(&mut self) -> io::Result<()> {
        // TODO: flush buffers if is not empty?
        self.stream.shutdown().await
    }

    pub async fn write<T: Serialize>(&mut self, value: &T) -> io::Result<usize> {
        to_buffer(value, &mut self.write_buffer)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let len = self.write_buffer.len();
        self.stream.write_all(&self.write_buffer).await?;
        self.write_buffer.advance(len);
        Ok(len)
    }

    pub async fn write_frame(&mut self, channel: ShortUint, frame: Frame) -> io::Result<usize> {
        // reserve bytes for frame header, which to be updated after encoding payload
        let header = FrameHeader {
            frame_type: frame.get_frame_type(),
            channel,
            payload_size: 0,
        };
        to_buffer(&header, &mut self.write_buffer).unwrap();

        // encode payload
        let payload_size = to_buffer(&frame, &mut self.write_buffer)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        // update frame's payload size
        for (i, v) in (payload_size as u32).to_be_bytes().iter().enumerate() {
            let p = self.write_buffer.get_mut(i + 3).unwrap();
            *p = *v;
        }

        // encode frame end byte
        to_buffer(&FRAME_END, &mut self.write_buffer).unwrap();

        // flush whole buffer
        self.stream.write_all(&self.write_buffer).await?;

        // discard sent data in write buffer
        let len = self.write_buffer.len();
        self.write_buffer.advance(len);

        Ok(len)
    }

    /// To support channels multiplex on one connection
    /// we need to return the channel id.
    /// Return :
    ///     (channel_id, Frame)
    pub async fn read_frame(&mut self) -> io::Result<(ShortUint, Frame)> {
        // TODO: handle network error, such as timeout, corrupted frame
        loop {
            let len = self.stream.read_buf(&mut self.read_buffer).await?;
            if len == 0 {
                if self.read_buffer.is_empty() {
                    //TODO: map to own error
                    return Err(io::Error::new(io::ErrorKind::Other, "peer shutdown"));
                } else {
                    //TODO: map to own error
                    return Err(io::Error::new(io::ErrorKind::Other, "connection failure"));
                }
            }
            // TODO: replace with tracing
            println!("number of bytes read from network {len}");
            // println!("{:02X?}", self.read_buffer.as_ref());
            // println!("{:?}", self.read_buffer);

            match Frame::decode(&self.read_buffer) {
                Ok((len, channel, frame)) => {
                    // discard parsed data in read buffer
                    self.read_buffer.advance(len);
                    return Ok((channel, frame));
                }
                Err(err) => match err {
                    crate::frame::Error::Incomplete => continue,
                    crate::frame::Error::Corrupted => {
                        // TODO: map this error to indicate connection to be shutdown
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "corrupted frame, should close the connection",
                        ));
                    }
                    crate::frame::Error::Other(_) => todo!(),
                },
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::Connection;
    use crate::frame::*;
    use tokio::runtime;

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

            // C: Close
            conn.write_frame(0, Close::default().into_frame())
                .await
                .unwrap();

            // S: CloseOk
            let close_ok = conn.read_frame().await.unwrap();
            println!("{close_ok:?}");
        })
    }
}
