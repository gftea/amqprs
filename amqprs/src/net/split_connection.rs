use crate::frame::{Frame, FrameHeader, FRAME_END};

use amqp_serde::{to_buffer, types::AmqpChannelId};
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

use super::Error;
type Result<T> = std::result::Result<T, Error>;
const DEFAULT_BUFFER_SIZE: usize = 8192;

pub struct SplitConnection {
    reader: BufReader,
    writer: BufWriter,
}
pub struct BufReader {
    stream: OwnedReadHalf,
    buffer: BytesMut,
}
pub struct BufWriter {
    stream: OwnedWriteHalf,
    buffer: BytesMut,
}

// Support to split socket connection into reader half and wirter half, which can be run in different tasks cocurrently
// Same interfaces to read/write packet before and after split.
impl SplitConnection {
    pub async fn open(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();

        let read_buffer = BytesMut::with_capacity(DEFAULT_BUFFER_SIZE);
        let write_buffer = BytesMut::with_capacity(DEFAULT_BUFFER_SIZE);

        Ok(Self {
            reader: BufReader {
                stream: reader,
                buffer: read_buffer,
            },
            writer: BufWriter {
                stream: writer,
                buffer: write_buffer,
            },
        })
    }

    // split connection into reader half and writer half
    pub fn into_split(self) -> (BufReader, BufWriter) {
        (self.reader, self.writer)
    }

    // to keep same read/write interfaces before and after connection split
    // below interfaces are forwarded to `BufferReader` and `BufferWriter` internally
    pub async fn close(self) -> Result<()> {
        self.reader.close().await;
        self.writer.close().await
    }

    pub async fn write<T: Serialize>(&mut self, value: &T) -> Result<usize> {
        self.writer.write(value).await
    }

    pub async fn write_frame(&mut self, channel: AmqpChannelId, frame: Frame) -> Result<usize> {
        self.writer.write_frame(channel, frame).await
    }

    pub async fn read_frame(&mut self) -> Result<ChannelFrame> {
        self.reader.read_frame().await
    }
}

impl BufWriter {
    // write any serializable value to socket
    pub async fn write<T: Serialize>(&mut self, value: &T) -> Result<usize> {
        to_buffer(value, &mut self.buffer)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let len = self.buffer.len();
        self.stream.write_all(&self.buffer).await?;
        self.buffer.advance(len);
        Ok(len)
    }

    // write a AMQP frame over a specific channel
    pub async fn write_frame(&mut self, channel: AmqpChannelId, frame: Frame) -> Result<usize> {
        // TODO: tracing
        println!("SENT: {}, {:?}", channel, frame);

        // reserve bytes for frame header, which to be updated after encoding payload
        let header = FrameHeader {
            frame_type: frame.get_frame_type(),
            channel,
            payload_size: 0,
        };
        to_buffer(&header, &mut self.buffer).unwrap();

        // encode payload
        let payload_size = to_buffer(&frame, &mut self.buffer)?;

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

    // The socket connection will be shutdown if writer half is shutdown
    pub async fn close(mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

type ChannelFrame = (AmqpChannelId, Frame);

impl BufReader {
    // try to decode a whole frame from the bufferred data.
    // If it is incomplete data, return None;
    // If the frame syntax is corrupted, return Error.
    async fn decode(&mut self) -> Result<Option<ChannelFrame>> {
        match Frame::decode(&self.buffer)? {
            Some((len, channel_id, frame)) => {
                // discard parsed data in read buffer
                self.buffer.advance(len);
                // TODO: tracing
                println!("RECV: {}, {:?}", channel_id, frame);
                Ok(Some((channel_id, frame)))
            }
            None => Ok(None),
        }
    }

    // Read a complete frame from socket connection, return channel id and decoded frame.
    pub async fn read_frame(&mut self) -> Result<ChannelFrame> {
        // check if there is remaining data in buffer to decode first
        let result = self.decode().await?;
        if let Some(frame) = result {
            return Ok(frame);
        }
        // incomplete frame data remains in buffer, read until a complete frame
        loop {
            let len = self.stream.read_buf(&mut self.buffer).await?;
            if len == 0 {
                if self.buffer.is_empty() {
                    return Err(Error::PeerShutdown);
                } else {
                    return Err(Error::Interrupted);
                }
            }
            // TODO:  tracing
            // println!("number of bytes read from network {len}");
            let result = self.decode().await?;
            match result {
                Some(frame) => return Ok(frame),
                None => continue,
            }
        }
    }

    // do nothing except consume the reader itself
    pub async fn close(self) {}
}

#[cfg(test)]
mod test {

    use super::SplitConnection;
    use crate::frame::*;    
    use tokio::sync::mpsc;

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
        let _start = rx_resp.recv().await.unwrap();

        // C: 'StartOk'
        let mut start_ok = StartOk::default();
        let auth_machanism = 2;
        match auth_machanism {
            1 => {
                // default: PLAIN
                tx_req
                    .send((CONN_CTRL_CHANNEL, start_ok.into_frame()))
                    .await
                    .unwrap();
            }
            2 => {

                start_ok.machanisms = "AMQPLAIN".try_into().unwrap();
                let user = "user";
                let password = "bitnami";
                let s = format!(
                    "\x05LOGIN\x53\x00\x00\x00\x04{user}\x08PASSWORD\x53\x00\x00\x00\x07{password}"
                );
                start_ok.response = s.try_into().unwrap();
                tx_req
                    .send((CONN_CTRL_CHANNEL, start_ok.into_frame()))
                    .await
                    .unwrap();
            }
            3 => {
                start_ok.machanisms = "RABBIT-CR-DEMO".try_into().unwrap();
                start_ok.response = "user".try_into().unwrap();
                tx_req
                .send((CONN_CTRL_CHANNEL, start_ok.into_frame()))
                .await
                .unwrap();

                // S: Secure
                rx_resp.recv().await.unwrap();
                
                // C: SecureOk
                let secure_ok = SecureOk { response: "My password is bitnami".try_into().unwrap() };
                tx_req.send((CONN_CTRL_CHANNEL, secure_ok.into_frame())).await.unwrap();

            }
            _ => unimplemented!(),
        }

        // S: 'Tune'
        let tune = rx_resp.recv().await.unwrap();

        // C: TuneOk
        let mut tune_ok = TuneOk::default();
        let tune = match tune.1 {
            Frame::Tune(_, v) => v,
            _ => panic!("wrong message"),
        };

        tune_ok.channel_max = tune.channel_max;
        tune_ok.frame_max = tune.frame_max;
        tune_ok.heartbeat = tune.heartbeat;

        tx_req
            .send((CONN_CTRL_CHANNEL, tune_ok.into_frame()))
            .await
            .unwrap();

        // C: Open
        let open = Open::default().into_frame();
        tx_req
            .send((CONN_CTRL_CHANNEL, open))
            .await
            .unwrap();

        // S: OpenOk
        let _open_ok = rx_resp.recv().await.unwrap();

        // C: Close
        tx_req
            .send((CONN_CTRL_CHANNEL, Close::default().into_frame()))
            .await
            .unwrap();

        // S: CloseOk
        let _close_ok = rx_resp.recv().await.unwrap();
    }

    #[tokio::test]
    async fn test_connection_open_close() {
        let mut connection = SplitConnection::open("localhost:5672").await.unwrap();

        connection.write(&ProtocolHeader::default()).await.unwrap();
        let (channel_id, frame) = connection.read_frame().await.unwrap();
        assert_eq!(CONN_CTRL_CHANNEL, channel_id);
        println!(" {frame:?}");
        connection
            .write_frame(channel_id, StartOk::default().into_frame())
            .await
            .unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_split_open_close() {
        let (mut reader, mut writer) = SplitConnection::open("localhost:5672")
            .await
            .unwrap()
            .into_split();

        writer.write(&ProtocolHeader::default()).await.unwrap();
        let (channel_id, frame) = reader.read_frame().await.unwrap();
        assert_eq!(CONN_CTRL_CHANNEL, channel_id);
        println!(" {frame:?}");
        writer
            .write_frame(channel_id, StartOk::default().into_frame())
            .await
            .unwrap();
        reader.close().await;
        writer.close().await.unwrap();
    }
}
