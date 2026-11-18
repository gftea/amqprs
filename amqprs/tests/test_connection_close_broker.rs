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

#[derive(Debug, Default)]
struct Counts {
    client_close: usize,
    client_close_ok: usize,
    server_close: usize,
    server_close_ok: usize,
    client_eof: bool,
    server_eof: bool,
    server_reset_after_exchange: bool,
}

impl Counts {
    fn exchange_complete(&self, mode: Mode) -> bool {
        match mode {
            Mode::Heartbeat => {
                self.client_close == 1
                    && self.client_close_ok == 0
                    && self.server_close == 0
                    && self.server_close_ok == 1
            }
            Mode::Server => {
                self.client_close == 0
                    && self.client_close_ok == 1
                    && self.server_close == 1
                    && self.server_close_ok == 0
            }
            Mode::Crossed => {
                self.client_close == 1
                    && self.client_close_ok == 1
                    && self.server_close == 1
                    && self.server_close_ok == 1
            }
            Mode::Disconnect => false,
        }
    }

    fn transport_ended(&self) -> bool {
        self.client_eof && (self.server_eof || self.server_reset_after_exchange)
    }
}

enum FrameRead {
    Frame(Vec<u8>),
    Eof,
    Reset,
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

async fn frame(input: &mut (impl AsyncRead + Unpin)) -> Result<FrameRead, Failure> {
    let mut header = [0; 7];
    // EOF/reset is classified only at a frame boundary. Any error after
    // consuming part of a frame remains an error, regardless of close state.
    match input.read(&mut header[..1]).await {
        Ok(0) => return Ok(FrameRead::Eof),
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {
            return Ok(FrameRead::Reset);
        }
        Err(error) => return Err(error.into()),
        Ok(_) => {}
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
    Ok(FrameRead::Frame(bytes))
}

fn method(frame: &[u8], method: u8) -> bool {
    frame.len() >= 12
        && frame[0] == 1
        && frame[1..3] == [0, 0]
        && frame[7..11] == [0, 10, 0, method]
}

struct ResetAfterPrefix(&'static [u8]);

impl AsyncRead for ResetAfterPrefix {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buffer: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if self.0.is_empty() {
            return std::task::Poll::Ready(Err(std::io::ErrorKind::ConnectionReset.into()));
        }
        let length = self.0.len().min(buffer.remaining());
        buffer.put_slice(&self.0[..length]);
        self.0 = &self.0[length..];
        std::task::Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn parser_refuses_reset_in_partial_header_or_body() {
    for prefix in [&[1, 0][..], &[1, 0, 0, 0, 0, 0, 4, 0, 10][..]] {
        assert!(frame(&mut ResetAfterPrefix(prefix)).await.is_err());
    }
}

#[tokio::test]
async fn parser_distinguishes_boundary_reset_from_eof() {
    assert!(matches!(
        frame(&mut ResetAfterPrefix(&[])).await.unwrap(),
        FrameRead::Reset
    ));
    assert!(matches!(
        frame(&mut tokio::io::empty()).await.unwrap(),
        FrameRead::Eof
    ));
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
        let diagnostics = proxy_counts.clone();
        let client_writer = cw.clone();
        let upstream_closing = client_closing.clone();
        let upstream = async move {
            let mut protocol = [0; 8];
            cr.read_exact(&mut protocol).await?;
            bw.write_all(&protocol).await?;
            loop {
                let bytes = match frame(&mut cr).await? {
                    FrameRead::Frame(bytes) => bytes,
                    FrameRead::Eof => break,
                    FrameRead::Reset => return Err("client reset before natural EOF".into()),
                };
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
                let close_ok = method(&bytes, 51);
                bw.write_all(&bytes).await?;
                if close_ok {
                    upstream_counts.lock().await.client_close_ok += 1;
                }
            }
            upstream_counts.lock().await.client_eof = true;
            bw.shutdown().await?;
            Ok::<_, Failure>(())
        };
        let downstream = async move {
            loop {
                let bytes = match frame(&mut br).await? {
                    FrameRead::Frame(bytes) => bytes,
                    FrameRead::Eof => {
                        proxy_counts.lock().await.server_eof = true;
                        break;
                    }
                    FrameRead::Reset => {
                        let mut counts = proxy_counts.lock().await;
                        // RabbitMQ can use SO_LINGER={true,0}. Linux then
                        // reports reset after its completed AMQP exchange.
                        // Never accept reset as a substitute for missing ACKs.
                        if !counts.exchange_complete(mode) {
                            return Err("broker reset before completing the close exchange".into());
                        }
                        counts.server_reset_after_exchange = true;
                        break;
                    }
                };
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
            cw.lock().await.shutdown().await?;
            Ok::<_, Failure>(())
        };
        if mode == Mode::Disconnect {
            tokio::select! { result = upstream => result, result = downstream => result }
        } else {
            if let Err(error) = tokio::try_join!(upstream, downstream) {
                eprintln!("proxy state before cleanup: {:?}", diagnostics.lock().await);
                return Err(error);
            }
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
    assert!(counts.transport_ended());
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
    assert!(counts.transport_ended());
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
    assert!(counts.transport_ended());
    Ok(())
}
