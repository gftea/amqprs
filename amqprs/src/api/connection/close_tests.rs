use super::*;
use tokio::{io::AsyncWriteExt, net::TcpListener, sync::Notify, time::timeout};

#[tokio::test]
async fn writer_loss_unblocks_reader_without_peer_eof() {
    let budget = std::time::Duration::from_secs(3);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let (client, peer) = timeout(budget, async {
        tokio::join!(SplitConnection::open(&address), listener.accept())
    })
    .await
    .unwrap();
    let (reader, writer) = client.unwrap().into_split();
    let (mut peer, _) = peer.unwrap();
    let (outgoing, receiver) = mpsc::channel(1);
    let (management, commands) = mpsc::channel(1);
    let (shutdown, mut notification) = broadcast::channel(1);
    let connection = Connection {
        shared: Arc::new(SharedConnectionInner {
            runtime: tokio::runtime::Handle::current(),
            server_properties: ServerProperties {
                capabilities: ServerCapabilities {
                    consumer_cancel_notify: false,
                    publisher_confirms: false,
                    consumer_priorities: false,
                    authentication_failure_close: false,
                    per_consumer_qos: false,
                    connection_blocked: false,
                    exchange_exchange_bindings: false,
                    basic_nack: false,
                    direct_reply_to: false,
                },
                product: String::new(),
                cluster_name: String::new(),
                version: String::new(),
            },
            connection_name: "writer-loss-test".into(),
            channel_max: 128,
            frame_max: 131072,
            heartbeat: 60,
            outgoing_tx: outgoing.clone(),
            conn_mgmt_tx: management,
            shutdown_subscriber: shutdown.clone(),
        }),
        is_open: Arc::new(AtomicBool::new(true)),
        _guard: None,
    };
    let handler = ReaderHandler::new(
        reader,
        connection.clone_no_drop_guard(),
        outgoing,
        commands,
        128,
        shutdown,
    );
    let entered = Arc::new(Notify::new());
    let reader_entered = entered.clone();
    let mut task = tokio::spawn(async move {
        reader_entered.notify_one();
        handler.run_until_shutdown(60).await;
    });
    timeout(budget, entered.notified()).await.unwrap();

    // Model the writer's exit while retaining both the peer and the client's
    // write half. No socket EOF or heartbeat timeout can establish success.
    connection.set_is_open(false);
    drop(receiver);
    timeout(budget, peer.write_all(&[8, 0, 0, 0, 0, 0, 0, 0xce]))
        .await
        .unwrap()
        .unwrap();
    let completed = timeout(budget, &mut task).await;
    if completed.is_err() {
        task.abort();
        let _ = task.await;
        panic!("writer loss must stop the reader without waiting for peer EOF");
    }
    completed.unwrap().unwrap();
    assert!(timeout(budget, notification.recv()).await.unwrap().unwrap());
    drop(writer);
    drop(peer);
}
