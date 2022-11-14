use tokio::sync::{broadcast, mpsc};

use super::{BufWriter, OutgoingMessage, ConnManagementCommand};

pub(super) struct WriterHandler {
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

    pub async fn run_until_shutdown(mut self) {
        loop {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    println!("WriterHandler received shutdown notification");
                    break;
                }
                Some((channel_id, frame)) = self.outgoing_rx.recv() => {
                    if let Err(err) = self.stream.write_frame(channel_id, frame).await {
                        println!("Failed to send frame over network, cause: {}", err);
                        break;
                    }
                }
                else => {
                    break;
                }
            }
        }
        // FIXME: should here send Close method to server?
        println!("Shutdown WriterHandler!");
    }
}
