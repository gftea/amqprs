//! Controlled AMQP peers make shutdown ordering independent of broker timing.
//! Every socket/protocol wait is bounded; no sleep establishes a race.

use amqprs::{
    callbacks::ConnectionCallback,
    connection::{Connection, OpenConnectionArguments},
    error::Error,
    Close,
};
use async_trait::async_trait;
use std::{error::Error as StdError, future::Future, sync::Arc, task::Poll, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Notify,
    task::JoinHandle,
    time::timeout,
};

type Failure = Box<dyn StdError + Send + Sync>;
const BUDGET: Duration = Duration::from_secs(5);

struct Peer(TcpStream);

impl Peer {
    async fn send(&mut self, method: u16, arguments: &[u8]) -> Result<(), Failure> {
        let mut payload = vec![0, 10]; // Connection class, channel zero.
        payload.extend_from_slice(&method.to_be_bytes());
        payload.extend_from_slice(arguments);
        let mut bytes = vec![1, 0, 0];
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payload);
        bytes.push(0xce);
        timeout(BUDGET, self.0.write_all(&bytes)).await??;
        Ok(())
    }

    async fn heartbeat(&mut self) -> Result<(), Failure> {
        timeout(BUDGET, self.0.write_all(&[8, 0, 0, 0, 0, 0, 0, 0xce])).await??;
        Ok(())
    }

    async fn method(&mut self) -> Result<u16, Failure> {
        timeout(BUDGET, async {
            loop {
                let mut header = [0; 7];
                self.0.read_exact(&mut header).await?;
                let size = u32::from_be_bytes(header[3..7].try_into()?) as usize;
                if size > 1024 * 1024 {
                    return Err("test peer frame exceeds limit".into());
                }
                let mut payload = vec![0; size + 1];
                self.0.read_exact(&mut payload).await?;
                assert_eq!(payload[size], 0xce);
                if header[0] == 8 {
                    continue;
                }
                assert_eq!(header[0], 1, "expected method frame");
                assert_eq!(&header[1..3], &[0, 0], "expected channel zero");
                assert!(size >= 4);
                assert_eq!(&payload[..2], &[0, 10], "expected Connection class");
                return Ok(u16::from_be_bytes(payload[2..4].try_into()?));
            }
        })
        .await?
    }

    async fn eof(&mut self) -> Result<(), Failure> {
        let mut byte = [0];
        assert_eq!(timeout(BUDGET, self.0.read(&mut byte)).await??, 0);
        Ok(())
    }
}

struct Task<T>(JoinHandle<T>);

impl<T> Task<T> {
    async fn finish(mut self) -> Result<T, Failure> {
        Ok(timeout(BUDGET, &mut self.0).await??)
    }
}
impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn connection() -> Result<(Connection, Peer), Failure> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let peer = Task(tokio::spawn(async move {
        let (socket, _) = listener.accept().await?;
        let mut peer = Peer(socket);
        let mut protocol = [0; 8];
        peer.0.read_exact(&mut protocol).await?;
        assert_eq!(&protocol, b"AMQP\0\0\x09\x01");
        // Start: version, empty properties, PLAIN mechanism and en_US locale.
        peer.send(10, b"\0\x09\0\0\0\0\0\0\0\x05PLAIN\0\0\0\x05en_US")
            .await?;
        assert_eq!(peer.method().await?, 11); // StartOk.
                                              // Tune: channel-max 128, frame-max 131072, heartbeat 60.
        peer.send(30, &[0, 128, 0, 2, 0, 0, 0, 60]).await?;
        assert_eq!(peer.method().await?, 31); // TuneOk.
        assert_eq!(peer.method().await?, 40); // Open.
        peer.send(41, &[0]).await?; // OpenOk.
        Ok::<_, Failure>(peer)
    }));
    let connection = timeout(
        BUDGET,
        Connection::open(&OpenConnectionArguments::new(
            "127.0.0.1",
            port,
            "guest",
            "guest",
        )),
    )
    .await??;
    Ok((connection, peer.finish().await??))
}

async fn poll_pending(future: impl Future) {
    tokio::pin!(future);
    std::future::poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
}

struct Callback {
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
#[async_trait]
impl ConnectionCallback for Callback {
    async fn close(&mut self, _: &Connection, _: Close) -> Result<(), Error> {
        Ok(())
    }
    async fn blocked(&mut self, _: &Connection, _: String) {
        self.entered.notify_one();
        self.release.notified().await;
    }
    async fn unblocked(&mut self, _: &Connection) {}
    async fn secret_updated(&mut self, _: &Connection) {}
}

async fn block_reader(connection: &Connection, peer: &mut Peer) -> Result<Arc<Notify>, Failure> {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    connection
        .register_callback(Callback {
            entered: entered.clone(),
            release: release.clone(),
        })
        .await?;
    peer.send(60, &[0]).await?; // Blocked, empty reason.
    timeout(BUDGET, entered.notified()).await?;
    Ok(release)
}

async fn closing(connection: &Connection) -> Result<(), Failure> {
    timeout(BUDGET, async {
        while connection.is_open() {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn normal_close_completes_handshake() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let close = Task(tokio::spawn(connection.close()));
    assert_eq!(peer.method().await?, 50);
    peer.send(51, &[]).await?;
    close.finish().await??;
    peer.eof().await
}

#[tokio::test]
async fn heartbeat_during_close_does_not_stop_reader() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let close = Task(tokio::spawn(connection.close()));
    assert_eq!(peer.method().await?, 50);
    peer.heartbeat().await?;
    peer.send(51, &[]).await?;
    close.finish().await??;
    peer.eof().await
}

#[tokio::test]
async fn callback_released_during_close_does_not_stop_reader() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let release = block_reader(&connection, &mut peer).await?;
    let observer = connection.clone();
    let close = Task(tokio::spawn(connection.close()));
    closing(&observer).await?;
    release.notify_one();
    assert_eq!(peer.method().await?, 50);
    peer.send(51, &[]).await?;
    close.finish().await??;
    peer.eof().await
}

#[tokio::test]
async fn cancelling_first_pending_close_poll_still_finishes_handshake() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let release = block_reader(&connection, &mut peer).await?;
    let observer = connection.clone();
    // Poll and drop before yielding: close's registration ACK cannot arrive
    // while its caller exists, because the callback holds the reader.
    poll_pending(connection.close()).await;
    assert!(!observer.is_open());
    release.notify_one();
    assert_eq!(peer.method().await?, 50);
    peer.send(51, &[]).await?;
    peer.eof().await
}

#[tokio::test]
async fn cancelling_close_after_request_allows_peer_shutdown() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let close = Task(tokio::spawn(connection.close()));
    assert_eq!(peer.method().await?, 50);
    close.0.abort();
    peer.heartbeat().await?;
    peer.send(51, &[]).await?;
    peer.eof().await
}

#[tokio::test]
async fn last_handle_drop_completes_handshake() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    drop(connection);
    assert_eq!(peer.method().await?, 50);
    peer.send(51, &[]).await?;
    peer.eof().await
}

#[tokio::test]
async fn server_close_is_acknowledged_before_socket_shutdown() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let shutdown = connection.listen_network_io_failure();
    tokio::pin!(shutdown);
    // Subscribe before the peer closes; preserve the future after one poll.
    std::future::poll_fn(|cx| {
        assert!(shutdown.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    peer.send(50, &[0, 200, 0, 0, 0, 0, 0]).await?;
    assert_eq!(peer.method().await?, 51);
    peer.eof().await?;
    assert!(!timeout(BUDGET, shutdown).await?);
    assert!(!connection.is_open());
    Ok(())
}

#[tokio::test]
async fn simultaneous_close_finishes_both_handshakes() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let close = Task(tokio::spawn(connection.close()));
    assert_eq!(peer.method().await?, 50);
    peer.send(50, &[0, 200, 0, 0, 0, 0, 0]).await?;
    peer.send(51, &[]).await?;
    assert_eq!(peer.method().await?, 51);
    close.finish().await??;
    peer.eof().await
}

#[tokio::test]
async fn notifications_after_close_do_not_run_callbacks() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let entered = Arc::new(Notify::new());
    connection
        .register_callback(Callback {
            entered: entered.clone(),
            release: Arc::new(Notify::new()),
        })
        .await?;
    let close = Task(tokio::spawn(connection.close()));
    assert_eq!(peer.method().await?, 50);
    peer.send(60, &[0]).await?;
    peer.send(51, &[]).await?;
    close.finish().await??;
    peer.eof().await?;
    assert!(timeout(Duration::ZERO, entered.notified()).await.is_err());
    Ok(())
}

#[tokio::test]
async fn eof_while_waiting_for_close_ok_returns_error_and_notifies_failure() -> Result<(), Failure>
{
    let (connection, mut peer) = connection().await?;
    let observer = connection.clone();
    let shutdown = observer.listen_network_io_failure();
    tokio::pin!(shutdown);
    std::future::poll_fn(|cx| {
        assert!(shutdown.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    let close = Task(tokio::spawn(connection.close()));
    assert_eq!(peer.method().await?, 50);
    drop(peer);
    assert!(close.finish().await?.is_err());
    assert!(timeout(BUDGET, shutdown).await?);
    Ok(())
}

#[tokio::test]
async fn cancelled_close_registration_survives_subsequent_network_failure() -> Result<(), Failure> {
    let (connection, mut peer) = connection().await?;
    let release = block_reader(&connection, &mut peer).await?;
    let observer = connection.clone();
    let shutdown = observer.listen_network_io_failure();
    tokio::pin!(shutdown);
    std::future::poll_fn(|cx| {
        assert!(shutdown.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    poll_pending(connection.close()).await;
    drop(peer);
    release.notify_one();
    assert!(
        timeout(BUDGET, shutdown).await?,
        "reader must notify failure rather than panic"
    );
    Ok(())
}
