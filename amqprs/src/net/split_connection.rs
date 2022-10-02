use crate::frame::{Frame, FrameHeader};

use amqp_serde::{
    constants::FRAME_END,
    to_buffer,
    types::{AmqpChannelId, ShortUint},
};
use bytes::{Buf, BytesMut};
use serde::Serialize;
use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

const DEFAULT_BUFFER_SIZE: usize = 8192;

pub struct SplitConnection {
    reader: BufferReader,
    writer: BufferWriter,
}
pub struct BufferReader {
    stream: OwnedReadHalf,
    buffer: BytesMut,
}
pub struct BufferWriter {
    stream: OwnedWriteHalf,
    buffer: BytesMut,
}

impl SplitConnection {
    /// open a splitable socket connection
    pub async fn open(addr: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();

        let read_buffer = BytesMut::with_capacity(DEFAULT_BUFFER_SIZE);
        let write_buffer = BytesMut::with_capacity(DEFAULT_BUFFER_SIZE);

        Ok(Self {
            reader: BufferReader {
                stream: reader,
                buffer: read_buffer,
            },
            writer: BufferWriter {
                stream: writer,
                buffer: write_buffer,
            },
        })
    }
    pub fn into_split(self) -> (BufferReader, BufferWriter) {
        (self.reader, self.writer)
    }
    /// forward to reader and writer
    pub async fn close(self) -> io::Result<()> {
        self.reader.close().await;
        self.writer.close().await
    }
    /// forward to writer
    pub async fn write<T: Serialize>(&mut self, value: &T) -> io::Result<usize> {
        self.writer.write(value).await
    }
    /// forward to writer
    pub async fn write_frame(&mut self, channel: AmqpChannelId, frame: Frame) -> io::Result<usize> {
        self.writer.write_frame(channel, frame).await
    }
    /// forward to reader
    pub async fn read_frame(&mut self) -> io::Result<(ShortUint, Frame)> {
        self.reader.read_frame().await
    }
}

impl BufferWriter {
    /// write any sequence of bytes
    pub async fn write<T: Serialize>(&mut self, value: &T) -> io::Result<usize> {
        to_buffer(value, &mut self.buffer)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let len = self.buffer.len();
        self.stream.write_all(&self.buffer).await?;
        self.buffer.advance(len);
        Ok(len)
    }
    /// write a frame over specific channel
    pub async fn write_frame(&mut self, channel: AmqpChannelId, frame: Frame) -> io::Result<usize> {
        // reserve bytes for frame header, which to be updated after encoding payload
        let header = FrameHeader {
            frame_type: frame.get_frame_type(),
            channel,
            payload_size: 0,
        };
        to_buffer(&header, &mut self.buffer).unwrap();

        // encode payload
        let payload_size = to_buffer(&frame, &mut self.buffer)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        // update frame's payload size
        for (i, v) in (payload_size as u32).to_be_bytes().iter().enumerate() {
            let p = self.buffer.get_mut(i + 3).unwrap();
            *p = *v;
        }

        // encode frame end byte
        to_buffer(&FRAME_END, &mut self.buffer).unwrap();

        // flush whole buffer
        self.stream.write_all(&self.buffer).await?;

        // discard sent data in write buffer
        let len = self.buffer.len();
        self.buffer.advance(len);

        Ok(len)
    }

    /// close the socket and consume
    pub async fn close(mut self) -> io::Result<()> {
        // TODO: flush buffers if is not empty?
        self.stream.shutdown().await
    }
}
impl BufferReader {
    /// To support channels multiplex on one connection
    /// we need to return the channel id.
    /// Return :
    ///     (channel_id, Frame)
    pub async fn read_frame(&mut self) -> io::Result<(ShortUint, Frame)> {
        // TODO: handle network error, such as timeout, corrupted frame
        loop {
            let len = self.stream.read_buf(&mut self.buffer).await?;
            if len == 0 {
                if self.buffer.is_empty() {
                    //TODO: map to own error
                    return Err(io::Error::new(io::ErrorKind::Other, "peer shutdown"));
                } else {
                    //TODO: map to own error
                    return Err(io::Error::new(io::ErrorKind::Other, "connection failure"));
                }
            }
            // TODO: replace with tracing
            // println!("number of bytes read from network {len}");
            // println!("{:02X?}", self.buffer.as_ref());
            // println!("{:?}", self.buffer);

            match Frame::decode(&self.buffer) {
                Ok(value) => {
                    match value {
                        Some((len, channel, frame)) => {
                            // discard parsed data in read buffer
                            self.buffer.advance(len);
                            return Ok((channel, frame));
                        }
                        None => continue,
                    }
                }
                Err(err) => match err {
                    crate::frame::Error::Corrupted =>
                    // TODO: map this error to indicate connection to be shutdown
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "corrupted frame, should close the connection",
                        ))
                    }
                    crate::frame::Error::Inner(msg) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "internal error, should close the connection",
                        ))
                    }
                },
            }
        }
    }
    /// consume the reader
    pub async fn close(self) {}
}

#[cfg(test)]
mod test {

    use super::SplitConnection;
    use crate::frame::*;
    use tokio::{runtime, sync::mpsc, time::sleep};

    fn new_runtime() -> runtime::Runtime {
        let rt = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt
    }
    #[tokio::test]
    async fn test_streaming_read_write() {
        let (tx_resp, mut rx_resp) = mpsc::channel(1024);
        let (tx_req, mut rx_req) = mpsc::channel(1024);

        let (mut reader, mut writer) = SplitConnection::open("localhost:5672")
            .await
            .unwrap()
            .into_split();
        // TODO: protocol header negotiation in connection level
        // do not need support channel multiplex, only done once per new connection
        writer.write(&ProtocolHeader::default()).await.unwrap();

        // simulate  using  messaging channel as buffer
        // request to channel over writer half
        tokio::spawn(async move {
            while let Some((channel_id, frame)) = rx_req.recv().await {
                writer.write_frame(channel_id, frame).await.unwrap();
            }
        });
        // response from channel over reader half
        tokio::spawn(async move {
            while let Ok((channel_id, frame)) = reader.read_frame().await {
                tx_resp.send((channel_id, frame)).await.unwrap();
            }
        });

        // S: 'Start'
        let start = rx_resp.recv().await.unwrap();
        println!(" {start:?}");

        // C: 'StartOk'
        let start_ok = StartOk::default().into_frame();
        tx_req.send((0, start_ok)).await.unwrap();

        // S: 'Tune'
        let tune = rx_resp.recv().await.unwrap();
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

        tx_req.send((0, tune_ok.into_frame())).await.unwrap();

        // C: Open
        let open = Open::default().into_frame();
        tx_req.send((0, open)).await.unwrap();

        // S: OpenOk
        let open_ok = rx_resp.recv().await.unwrap();
        println!("{open_ok:?}");

        // C: Close
        tx_req
            .send((0, Close::default().into_frame()))
            .await
            .unwrap();

        // S: CloseOk
        let close_ok = rx_resp.recv().await.unwrap();
        println!("{close_ok:?}");
    }

    #[tokio::test]
    async fn test_connection_open_close() {
        let mut connection = SplitConnection::open("localhost:5672").await.unwrap();

        connection.write(&ProtocolHeader::default()).await.unwrap();
        let (channel_id, frame) = connection.read_frame().await.unwrap();
        assert_eq!(0, channel_id);
        println!(" {frame:?}");
        connection.write_frame(channel_id, StartOk::default().into_frame()).await.unwrap();
        connection.close().await;
    }

    #[tokio::test]
    async fn test_split_open_close() {
        let (mut reader, mut writer) = SplitConnection::open("localhost:5672").await.unwrap().into_split();

        writer.write(&ProtocolHeader::default()).await.unwrap();
        let (channel_id, frame) = reader.read_frame().await.unwrap();
        assert_eq!(0, channel_id);
        println!(" {frame:?}");
        writer.write_frame(channel_id, StartOk::default().into_frame()).await.unwrap();
        reader.close().await;
        writer.close().await;

    }
}
