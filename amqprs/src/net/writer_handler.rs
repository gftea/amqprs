use tokio::sync::{broadcast, mpsc::Receiver};

use super::{BufWriter, OutgoingMessage};

pub(super) struct WriterHandler {
    stream: BufWriter,
    /// receiver half to forward outgoing messages from AMQ connection/channel to server
    forwarder: Receiver<OutgoingMessage>,
    /// listener of shutdown signal
    shutdown_listener: broadcast::Receiver<()>,
}

impl WriterHandler {
    pub fn new(
        stream: BufWriter,
        forwarder: Receiver<OutgoingMessage>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            stream,
            forwarder,
            shutdown_listener: shutdown,
        }
    }

    pub async fn run_until_shutdown(mut self) {
        loop {
            tokio::select! {
                _ = self.shutdown_listener.recv() => {
                    println!("received shutdown");
                    break;
                }
                Some((channel_id, frame)) = self.forwarder.recv() => {
                    if let Err(_) = self.stream.write_frame(channel_id, frame).await {
                        break;
                    }

                }
                else => {
                    break;
                }
            }
        }
        // TODO: send Close method to server?
        println!("shutdown writer handler!");
    }
}
