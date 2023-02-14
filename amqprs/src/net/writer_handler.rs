use amqp_serde::types::ShortUint;
use tokio::{
    sync::{broadcast, mpsc},
    task::yield_now,
    time,
};
#[cfg(feature = "traces")]
use tracing::{debug, error, info, trace};

use crate::{
    connection::Connection,
    frame::{Frame, HeartBeat, DEFAULT_CONN_CHANNEL},
};

use super::{BufIoWriter, OutgoingMessage};

pub(crate) struct WriterHandler {
    stream: BufIoWriter,
    /// receiver half to forward outgoing messages from AMQ connection/channel to server
    outgoing_rx: mpsc::Receiver<OutgoingMessage>,
    /// listener of shutdown signal
    shutdown: broadcast::Receiver<()>,
    /// connection
    amqp_connection: Connection,
}

impl WriterHandler {
    pub fn new(
        stream: BufIoWriter,
        outgoing_rx: mpsc::Receiver<OutgoingMessage>,
        shutdown: broadcast::Receiver<()>,
        amqp_connection: Connection,
    ) -> Self {
        Self {
            stream,
            outgoing_rx,
            shutdown,
            amqp_connection,
        }
    }

    pub async fn run_until_shutdown(mut self, heartbeat: ShortUint) {
        // to take in acount network delay and congestion
        // heartbeat should be sent at a interval of timeout / 2
        let interval: u64 = (heartbeat / 2).into();
        let mut expiration = time::Instant::now() + time::Duration::from_secs(interval);

        loop {
            tokio::select! {
                biased;

                channel_frame = self.outgoing_rx.recv() => {
                    let (channel_id, frame) = match channel_frame {
                        None => break,
                        Some(v) => v,
                    };
                    if let Err(err) = self.stream.write_frame(channel_id, frame, self.amqp_connection.frame_max()).await {
                        #[cfg(feature="tracing")]
                        error!("failed to send frame over connection {}, cause: {}", self.amqp_connection, err);
                        break;
                    }
                    expiration = time::Instant::now() + time::Duration::from_secs(interval);
                    #[cfg(feature="tracing")]
                    trace!("connection {} heartbeat deadline is updated to {:?}", self.amqp_connection, expiration);
                }
                _ = time::sleep_until(expiration) => {
                    if expiration <= time::Instant::now() {
                        expiration = time::Instant::now() + time::Duration::from_secs(interval);

                        if let Err(err) = self.stream.write_frame(DEFAULT_CONN_CHANNEL, Frame::HeartBeat(HeartBeat), self.amqp_connection.frame_max()).await {
                            #[cfg(feature="tracing")]
                            error!("failed to send heartbeat over connection {}, cause: {}", self.amqp_connection, err);
                            break;
                        }
                        #[cfg(feature="tracing")]
                        debug!("sent heartbeat over connection {}", self.amqp_connection,);
                    }
                }
                _ = self.shutdown.recv() => {
                    #[cfg(feature="tracing")]
                    info!("received shutdown notification for connection {}", self.amqp_connection);
                    // try to give last chance for last message.
                    yield_now().await;
                    break;
                }
                else => {
                    break;
                }
            }
        }
        self.amqp_connection.set_is_open(false);

        if let Err(err) = self.stream.close().await {
            #[cfg(feature = "traces")]
            error!(
                "failed to close i/o writer of connection {}, cause: {}",
                self.amqp_connection, err
            );
        }
    }
}
