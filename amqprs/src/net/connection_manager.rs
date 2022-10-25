use amqp_serde::types::ShortUint;
use tokio::sync::{broadcast, mpsc};

use super::{
    reader_handler::ReaderHandler, writer_handler::WriterHandler, InternalChannels, SplitConnection,
};

//  TODO: to be part of static configuration
pub(crate) const OUTGOING_MESSAGE_CHANNEL_BUFFER_SIZE: usize = 64;
pub(crate) const MANAGEMENT_CHANNEL_BUFFER_SIZE: usize = 32;

/// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
pub(crate) async fn spawn(connection: SplitConnection, channel_max: ShortUint) -> InternalChannels {
    // The Connection Manager will Spawn two  tasks for connection
    // - one task for writer handler
    // - one task for reader handler
    let (outgoing_tx, outgoing_rx) = mpsc::channel(OUTGOING_MESSAGE_CHANNEL_BUFFER_SIZE);
    let (mgmt_tx, mgmt_rx) = mpsc::channel(MANAGEMENT_CHANNEL_BUFFER_SIZE);

    let (shutdown_notifer, shutdown_listener) = broadcast::channel::<()>(1);

    let (reader, writer) = connection.into_split();

    // spawn task for read connection hanlder
    let rh = ReaderHandler::new(
        reader,
        outgoing_tx.clone(),
        mgmt_rx,
        channel_max,
        shutdown_notifer,
    );
    tokio::spawn(async move {
        rh.run_until_shutdown().await;
    });
    // spawn task for write connection handler
    let wh = WriterHandler::new(writer, outgoing_rx, shutdown_listener);
    tokio::spawn(async move {
        wh.run_until_shutdown().await;
    });

    InternalChannels {
        outgoing_tx,
        mgmt_tx,
    }
}
