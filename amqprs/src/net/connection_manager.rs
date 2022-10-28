use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{broadcast, mpsc, oneshot};

use super::{
    reader_handler::ReaderHandler, writer_handler::WriterHandler, ChannelResource,
    ManagementCommand, OutgoingMessage, RegisterChannelResource, SplitConnection,
};

/// It spawns tasks for `WriterHandler` and `ReaderHandler` to handle outgoing/incoming messages cocurrently.
pub(crate) async fn spawn_handlers(
    connection: SplitConnection,
    channel_max: ShortUint,
    outgoing_tx: mpsc::Sender<OutgoingMessage>,
    outgoing_rx: mpsc::Receiver<OutgoingMessage>,
    _mgmt_tx: mpsc::Sender<ManagementCommand>,
    mgmt_rx: mpsc::Receiver<ManagementCommand>,
) {
    // The Connection Manager will Spawn two  tasks for connection
    // - one task for writer handler
    // - one task for reader handler

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
}

pub(crate) async fn register_channel_resource(
    mgmt_tx: &mpsc::Sender<ManagementCommand>,
    channel_id: Option<AmqpChannelId>,
    resource: ChannelResource,
) -> Option<AmqpChannelId> {
    // allocate channel for receiving incoming message from server
    // register the sender half to handler, and keep the receiver half
    let (acker, resp) = oneshot::channel();
    let cmd = ManagementCommand::RegisterChannelResource(RegisterChannelResource {
        channel_id,
        resource,
        acker,
    });

    // register responder for the channel.
    // If no channel id is given, it will be allocated by management task and included in acker response
    // otherwise same id will be received in response
    if let Err(err) = mgmt_tx.send(cmd).await {
        println!("register channel resource failure, cause: {}", err);
        return None;
    }

    // expect a channel id in response
    match resp.await {
        Ok(res) => {
            if let None = res {
                println!("register channel resource failure, failed to allocate channel id");
            }
            res
        }
        Err(err) => {
            println!("register channel resource failure, cause: {}", err);
            None
        }
    }
}
