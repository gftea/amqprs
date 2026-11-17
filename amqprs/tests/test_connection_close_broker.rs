//! Real-broker checks use the same plaintext endpoint and credentials as the
//! repository's integration fixture. AMQPRS_TEST_PORT can select another port.

use amqprs::{
    callbacks::DefaultConnectionCallback,
    connection::{Connection, OpenConnectionArguments},
};
use std::{error::Error, future::Future, sync::Arc, task::Poll, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, Notify},
    task::JoinHandle,
    time::timeout,
};

type Failure = Box<dyn Error + Send + Sync>;
const BUDGET: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Heartbeat,
    Crossed,
    Server,
    Disconnect,
}

#[derive(Default)]
struct Counts {
    client_close: usize,
    client_close_ok: usize,
    server_close: usize,
    server_close_ok: usize,
    client_eof: bool,
    server_eof: bool,
}

struct Proxy {
    task: JoinHandle<Result<(), Failure>>,
    counts: Arc<Mutex<Counts>>,
    server_closing: Arc<Notify>,
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn frame(input: &mut (impl AsyncRead + Unpin)) -> Result<Option<Vec<u8>>, Failure> {
    let mut header = [0; 7];
    // EOF is normal only at a frame boundary, not in a partial frame.
    if input.read(&mut header[..1]).await? == 0 {
        return Ok(None);
    }
    input.read_exact(&mut header[1..]).await?;
    let size = u32::from_be_bytes(header[3..7].try_into()?) as usize;
    if size > 1024 * 1024 {
        return Err("proxy frame exceeds limit".into());
    }
    let mut bytes = header.to_vec();
    bytes.resize(size + 8, 0);
    input.read_exact(&mut bytes[7..]).await?;
    assert_eq!(bytes[size + 7], 0xce);
    Ok(Some(bytes))
}

fn method(frame: &[u8], method: u8) -> bool {
    frame.len() >= 12
        && frame[0] == 1
        && frame[1..3] == [0, 0]
        && frame[7..11] == [0, 10, 0, method]
}

async fn connect(mode: Mode) -> Result<(Connection, Proxy), Failure> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let broker_port = std::env::var("AMQPRS_TEST_PORT")
        .unwrap_or_else(|_| "5672".into())
        .parse::<u16>()?;
    let counts = Arc::new(Mutex::new(Counts::default()));
    let proxy_counts = counts.clone();
    let server_closing = Arc::new(Notify::new());
    let proxy_closing = server_closing.clone();
    let client_closing = Arc::new(Notify::new());
    let task = tokio::spawn(async move {
        let (client, _) = listener.accept().await?;
        let broker = TcpStream::connect(("127.0.0.1", broker_port)).await?;
        let (mut cr, cw) = client.into_split();
        let (mut br, mut bw) = broker.into_split();
        let cw = Arc::new(Mutex::new(cw));
        let upstream_counts = proxy_counts.clone();
        let client_writer = cw.clone();
        let upstream_closing = client_closing.clone();
        let upstream = async move {
            let mut protocol = [0; 8];
            cr.read_exact(&mut protocol).await?;
            bw.write_all(&protocol).await?;
            while let Some(bytes) = frame(&mut cr).await? {
                if method(&bytes, 50) {
                    upstream_counts.lock().await.client_close += 1;
                    if mode == Mode::Heartbeat {
                        client_writer
                            .lock()
                            .await
                            .write_all(&[8, 0, 0, 0, 0, 0, 0, 0xce])
                            .await?;
                    }
                    if mode == Mode::Disconnect {
                        return Ok::<_, Failure>(());
                    }
                    bw.write_all(&bytes).await?;
                    upstream_closing.notify_one();
                    continue;
                }
                if method(&bytes, 51) {
                    upstream_counts.lock().await.client_close_ok += 1;
                }
                bw.write_all(&bytes).await?;
            }
            upstream_counts.lock().await.client_eof = true;
            bw.shutdown().await?;
            Ok::<_, Failure>(())
        };
        let downstream = async move {
            while let Some(bytes) = frame(&mut br).await? {
                if method(&bytes, 50) {
                    proxy_counts.lock().await.server_close += 1;
                    if mode == Mode::Crossed {
                        // Hold the broker's Close until the client has sent its
                        // own Close. Both genuine requests then cross in flight.
                        proxy_closing.notify_one();
                        client_closing.notified().await;
                    }
                }
                if method(&bytes, 51) {
                    proxy_counts.lock().await.server_close_ok += 1;
                }
                cw.lock().await.write_all(&bytes).await?;
            }
            proxy_counts.lock().await.server_eof = true;
            cw.lock().await.shutdown().await?;
            Ok::<_, Failure>(())
        };
        if mode == Mode::Disconnect {
            tokio::select! { result = upstream => result, result = downstream => result }
        } else {
            tokio::try_join!(upstream, downstream)?;
            Ok(())
        }
    });
    let proxy = Proxy {
        task,
        counts,
        server_closing,
    };
    let connection = timeout(
        BUDGET,
        Connection::open(&OpenConnectionArguments::new(
            "127.0.0.1",
            port,
            "user",
            "bitnami",
        )),
    )
    .await??;
    connection
        .register_callback(DefaultConnectionCallback)
        .await?;
    Ok((connection, proxy))
}

#[tokio::test]
async fn real_broker_heartbeat_during_close_completes_handshake() -> Result<(), Failure> {
    let (connection, mut proxy) = connect(Mode::Heartbeat).await?;
    timeout(BUDGET, connection.close()).await??;
    timeout(BUDGET, &mut proxy.task).await???;
    let counts = proxy.counts.lock().await;
    assert_eq!(counts.client_close, 1);
    assert_eq!(counts.server_close_ok, 1);
    assert!(counts.client_eof && counts.server_eof);
    Ok(())
}

#[tokio::test]
async fn real_broker_crossed_close_responds_to_both_requests() -> Result<(), Failure> {
    let (connection, mut proxy) = connect(Mode::Crossed).await?;
    // Connection.Blocked is server-to-client. Sending it to RabbitMQ provokes
    // a connection-level protocol exception, whose genuine Close is gated.
    connection.blocked("provoke connection close").await?;
    timeout(BUDGET, proxy.server_closing.notified()).await?;
    timeout(BUDGET, connection.close()).await??;
    timeout(BUDGET, &mut proxy.task).await???;
    let counts = proxy.counts.lock().await;
    assert_eq!(counts.client_close, 1);
    assert_eq!(counts.server_close, 1);
    assert_eq!(counts.client_close_ok, 1);
    assert_eq!(counts.server_close_ok, 1);
    assert!(counts.client_eof && counts.server_eof);
    Ok(())
}

#[tokio::test]
async fn real_broker_connection_loss_during_close_remains_an_error() -> Result<(), Failure> {
    let (connection, mut proxy) = connect(Mode::Disconnect).await?;
    let observer = connection.clone();
    let failed = observer.listen_network_io_failure();
    tokio::pin!(failed);
    std::future::poll_fn(|cx| {
        assert!(failed.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(timeout(BUDGET, connection.close()).await?.is_err());
    assert!(timeout(BUDGET, failed).await?);
    timeout(BUDGET, &mut proxy.task).await???;
    assert_eq!(proxy.counts.lock().await.client_close, 1);
    Ok(())
}

#[tokio::test]
async fn real_broker_server_close_is_acknowledged() -> Result<(), Failure> {
    let (connection, mut proxy) = connect(Mode::Server).await?;
    connection
        .blocked("provoke server-only connection close")
        .await?;
    timeout(BUDGET, &mut proxy.task).await???;
    assert!(!connection.is_open());
    let counts = proxy.counts.lock().await;
    assert_eq!(counts.client_close, 0);
    assert_eq!(counts.server_close, 1);
    assert_eq!(counts.client_close_ok, 1);
    assert_eq!(counts.server_close_ok, 0);
    assert!(counts.client_eof && counts.server_eof);
    Ok(())
}
