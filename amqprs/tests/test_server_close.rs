use amqprs::{
    channel::{BasicAckArguments, BasicPublishArguments, QueueDeclareArguments},
    connection::Connection,
    BasicProperties,
};
use tokio::time;
mod common;

/// A server-initiated close releases the channel ID, so a retained
/// handle must not send frames on the replacement channel that reuses it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_server_closed_handle_does_not_use_recycled_channel_id() {
    common::setup_logging();

    let args = common::build_conn_args();
    let connection = Connection::open(&args).await.unwrap();
    let stale = connection.open_channel(None).await.unwrap();
    let stale_id = stale.channel_id();

    let missing = QueueDeclareArguments::default()
        .queue("amqprs.test.missing-queue.server-close".to_owned())
        .passive(true)
        .finish();
    assert!(stale.queue_declare(missing).await.is_err());
    time::timeout(time::Duration::from_secs(5), async {
        while stale.is_open() {
            time::sleep(time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("server close must mark the channel closed");

    let replacement = time::timeout(time::Duration::from_secs(5), async {
        loop {
            let channel = connection.open_channel(None).await.unwrap();
            if channel.channel_id() == stale_id {
                return channel;
            }
            time::sleep(time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("released channel ID must be reused");

    assert!(stale
        .basic_publish(
            BasicProperties::default(),
            b"stale".to_vec(),
            BasicPublishArguments::new("", "amqprs.test.unused"),
        )
        .await
        .is_err());
    assert!(stale
        .basic_ack(BasicAckArguments::new(1, false))
        .await
        .is_err());

    // A stale ack reaching the broker would close the replacement channel
    // with PRECONDITION_FAILED; a synchronous request confirms it is intact.
    replacement.flow(true).await.unwrap();
    assert!(replacement.is_open());

    replacement.close().await.unwrap();
    connection.close().await.unwrap();
}

/// Repeated broker closes must not exhaust the negotiated channel ID range.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_missing_queue_probes_do_not_exhaust_channel_ids() {
    let args = common::build_conn_args();
    let connection = Connection::open(&args).await.unwrap();
    let live = connection.open_channel(None).await.unwrap();
    for _ in 0..4096 {
        let probe = connection.open_channel(None).await.unwrap();
        assert!(probe
            .queue_declare(
                QueueDeclareArguments::default()
                    .queue("amqprs.test.missing-queue.id-exhaustion".to_owned())
                    .passive(true)
                    .finish(),
            )
            .await
            .is_err());
        live.flow(true).await.unwrap();
    }
    assert!(connection.is_open());
    live.close().await.unwrap();
    connection.close().await.unwrap();
}
