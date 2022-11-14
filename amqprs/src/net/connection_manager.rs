use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{broadcast, mpsc, oneshot};

use super::{
    reader_handler::ReaderHandler, writer_handler::WriterHandler, ChannelResource,
    ConnManagementCommand, OutgoingMessage, RegisterChannelResource, SplitConnection,
};

/// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
pub(crate) async fn spawn_handlers(
    connection: SplitConnection,
    channel_max: ShortUint,
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    outgoing_rx: mpsc::Receiver<OutgoingMessage>,
    _conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
    conn_mgmt_rx: mpsc::Receiver<ConnManagementCommand>,
) {
    // Spawn two tasks for the connection
    // - one task for writer
    // - one task for reader

    let (shutdown_notifer, shutdown_listener) = broadcast::channel::<()>(1);

    let (reader, writer) = connection.into_split();

    // spawn task for read connection handler
    let rh = ReaderHandler::new(
        reader,
        outgoing_tx.clone(),
        conn_mgmt_rx,
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
}

pub(crate) async fn register_channel_resource(
    conn_mgmt_tx: &mpsc::Sender<ConnManagementCommand>,
    channel_id: Option<AmqpChannelId>,
    resource: ChannelResource,
) -> Option<AmqpChannelId> {
    let (acker, acker_rx) = oneshot::channel();
    let cmd = ConnManagementCommand::RegisterChannelResource(RegisterChannelResource {
        channel_id,
        resource,
        acker,
    });

    // If no channel id is given, it will be allocated by management task and included in acker response
    // otherwise same id will be received in response
    if let Err(err) = conn_mgmt_tx.send(cmd).await {
        println!("Failed to register channel resource, cause: {}", err);
        return None;
    }

    // expect a channel id in response
    match acker_rx.await {
        Ok(res) => {
            if let None = res {
                println!("Failed to register channel resource, error in channel id allocation");
            }
            res
        }
        Err(err) => {
            println!("Failed to register channel resource, cause: {}", err);
            None
        }
    }
}
