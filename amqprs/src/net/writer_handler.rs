

use amqp_serde::types::ShortUint;
use tokio::{
    sync::{broadcast, mpsc},
    time, task::yield_now,
};
use tracing::{debug, error, info, trace};

use crate::frame::{Frame, HeartBeat, DEFAULT_CONN_CHANNEL};

use super::{BufWriter, OutgoingMessage};

pub(crate) struct WriterHandler {
    stream: BufWriter,
    /// receiver half to forward outgoing messages from AMQ connection/channel to server
    outgoing_rx: mpsc::Receiver<OutgoingMessage>,
    /// listener of shutdown signal
    shutdown: broadcast::Receiver<()>,
}

impl WriterHandler {
    pub fn new(
        stream: BufWriter,
        outgoing_rx: mpsc::Receiver<OutgoingMessage>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            stream,
            outgoing_rx,
            shutdown,
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
                    if let Err(err) = self.stream.write_frame(channel_id, frame).await {
                        error!("failed to send frame over network, cause: {}!", err);
                        break;
                    }
                    expiration = time::Instant::now() + time::Duration::from_secs(interval);
                    trace!("client heartbeat deadline is updated to {:?}", expiration);
                }
                _ = time::sleep_until(expiration) => {
                    if expiration <= time::Instant::now() {
                        expiration = time::Instant::now() + time::Duration::from_secs(interval);

                        if let Err(err) = self.stream.write_frame(DEFAULT_CONN_CHANNEL, Frame::HeartBeat(HeartBeat)).await {
                            error!("failed to send heartbeat over network, cause: {}!", err);
                            break;
                        }
                        debug!("sent heartbeat ...");
                    }

                }
                _ = self.shutdown.recv() => {
                    info!("received shutdown notification.");
                    // try to give last chance for last message.
                    yield_now().await;
                    break;
                }
                else => {
                    break;
                }
            }
        }
        if let Err(err) = self.stream.close().await {
            error!("failed to close writer cleanly, cause: {}", err);
        }
    }
}
