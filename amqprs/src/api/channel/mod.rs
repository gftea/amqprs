//! Implementation of AMQP_0-9-1's Channel class compatible with RabbitMQ.
//!
//! It provides [`APIs`] to manage an AMQP [`Channel`].
//!
//! User should hold the channel object until no longer needs it, and call the [`close`] method
//! to gracefully shutdown the channel.
//!
//! When channel object is dropped, it will try with best effort
//! to close the channel, but no guarantee to handle close errors.
//!
//! Almost all methods of [`Channel`] accepts arguments, this module also contains
//! all argument types for each method.
//!
//! # Example
//! See [`crate`] documentation for quick start.
//! See details in documentation of each method.
//!
//! [`APIs`]: struct.Channel.html#implementations
//! [`Channel`]: struct.Channel.html
//! [`close`]: struct.Channel.html#method.close
//!
use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use amqp_serde::types::AmqpChannelId;
use tokio::sync::{
    mpsc::{self, error::SendError},
    oneshot, Mutex,
};

use super::callbacks::ChannelCallback;
use crate::{
    api::{error::Error, Result},
    connection::Connection,
    frame::{CloseChannel, CloseChannelOk, Deliver, Flow, FlowOk, Frame, MethodHeader, Return},
    net::{ConnManagementCommand, IncomingMessage, OutgoingMessage},
    BasicProperties,
};
#[cfg(feature = "traces")]
use tracing::{error, info, trace};

/// Combined message received by a consumer
///
/// Although all the fields are `Option<T>` type, the library guarantee
/// when user gets a message from receiver half of a consumer,
/// all the fields have value of `Some<T>`.
pub struct ConsumerMessage {
    pub deliver: Option<Deliver>,
    pub basic_properties: Option<BasicProperties>,
    pub content: Option<Vec<u8>>,
    remaining: usize,
}

/// Message buffer for a `Return + content` sequence from server.
pub(crate) struct ReturnMessage {
    ret: Option<Return>,
    basic_properties: Option<BasicProperties>,
    content: Option<Vec<u8>>,
    remaining: usize,
}

/// Message buffer for a `GetOk + content` sequence from server.
pub(crate) struct GetOkMessage {
    content: Option<Vec<u8>>,
    remaining: usize,
}

/// Command to register consumer of asynchronous delivered contents.
pub(crate) struct RegisterContentConsumer {
    consumer_tag: String,
    consumer_tx: mpsc::UnboundedSender<ConsumerMessage>,
}

/// Command to deregister consumer of asynchronous delivered contents.
///
/// Consumer should be deregistered when it is cancelled or the channel is closed.
pub(crate) struct DeregisterContentConsumer {
    consumer_tag: String,
}

/// Command to register sender to forward server's response to `get` request.
///
/// Server will respond `get-ok` + `message propertities` + `content body` in sequence,
/// so the sender should be mpsc instead of oneshot.
pub(crate) struct RegisterGetContentResponder {
    tx: mpsc::UnboundedSender<IncomingMessage>,
}

/// Command to register oneshot sender for response from server.
pub(crate) struct RegisterOneshotResponder {
    pub method_header: &'static MethodHeader,
    /// oneshot sender to forward response message from server.
    pub responder: oneshot::Sender<IncomingMessage>,
    // oneshot sender to acknowledge registration is done.
    pub acker: oneshot::Sender<()>,
}

/// Command to register channel callbacks
pub(crate) struct RegisterChannelCallback {
    pub callback: Box<dyn ChannelCallback + Send + 'static>,
}

/// List of management commands for channel dispatcher.
pub(crate) enum DispatcherManagementCommand {
    RegisterContentConsumer(RegisterContentConsumer),
    DeregisterContentConsumer(DeregisterContentConsumer),
    RegisterGetContentResponder(RegisterGetContentResponder),
    RegisterOneshotResponder(RegisterOneshotResponder),
    RegisterChannelCallback(RegisterChannelCallback),
}

/// Type represents an AMQP Channel.
///
/// First, create a new AMQP channel by `Connection's` method [`Connection::open_channel`].
///
/// Second, register callbacks for the channel by [`Channel::register_callback`].
///
/// Then, the channel is ready to use.
///
/// # Concurrency
///
/// It is not recommended to share same `Channel` object between tasks/threads, because it allows
/// interleaving the AMQP protocol messages in same channel in concurrent setup.
/// Applications should be aware of the this limitation in AMQP protocol itself.
///
/// See detailed explanation in [`Java Client`], it applies to the library also.
///
/// [`Connection::open_channel`]: ../connection/struct.Connection.html#method.open_channel
/// [`Channel::register_callback`]: struct.Channel.html#method.register_callback
/// [`Java Client`]: https://www.rabbitmq.com/api-guide.html#concurrency
#[derive(Clone)]
pub struct Channel {
    shared: Arc<SharedChannelInner>,
    /// associated connection
    connection: Connection,
    /// drop guard to close channel when dropped
    _guard: Option<Arc<DropGuard>>,
}

struct DropGuard(Arc<SharedChannelInner>);

pub(crate) struct SharedChannelInner {
    /// open state
    is_open: Arc<AtomicBool>,
    /// channel id
    channel_id: AmqpChannelId,
    /// tx half to send message to `WriteHandler` task
    outgoing_tx: ChannelOutgoingTx,
    /// tx half to send managment command to `ReaderHandler` task
    conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
    /// tx half to send management command to `ChannelDispatcher` task
    dispatcher_mgmt_tx: mpsc::UnboundedSender<DispatcherManagementCommand>,
}

/// Sender of a channel's outgoing frames that rejects frames once the channel
/// is closed.
///
/// A closed channel's ID is released and may be reused by a newly opened
/// channel, so a retained handle must not send frames under that ID.
pub(crate) struct ChannelOutgoingTx {
    // Admission stays locked until enqueue finishes. Closing takes the same
    // lock before changing state, so every admitted frame precedes CloseOk
    // in the writer queue and therefore precedes any reuse of the ID.
    admission: Mutex<()>,
    is_open: Arc<AtomicBool>,
    tx: mpsc::Sender<OutgoingMessage>,
}

impl ChannelOutgoingTx {
    pub(crate) async fn send(
        &self,
        message: OutgoingMessage,
    ) -> std::result::Result<(), SendError<OutgoingMessage>> {
        let _admission = self.admission.lock().await;
        if !self.is_open.load(Ordering::Relaxed) {
            return Err(SendError(message));
        }
        self.tx.send(message).await
    }

    pub(crate) fn blocking_send(
        &self,
        message: OutgoingMessage,
    ) -> std::result::Result<(), SendError<OutgoingMessage>> {
        let _admission = self.admission.blocking_lock();
        if !self.is_open.load(Ordering::Relaxed) {
            return Err(SendError(message));
        }
        self.tx.blocking_send(message)
    }

    async fn mark_closed(&self) -> bool {
        let _admission = self.admission.lock().await;
        self.is_open.swap(false, Ordering::AcqRel)
    }

    /// Sends regardless of open state, for close handshake frames and replies
    /// to the server while the channel ID is still registered.
    pub(crate) fn unchecked(&self) -> &mpsc::Sender<OutgoingMessage> {
        &self.tx
    }
}

impl SharedChannelInner {
    /// Register oneshot responder for single message.
    ///
    /// Used for synchronous request/response protocol.
    async fn register_responder(
        &self,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<IncomingMessage>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterOneshotResponder {
            method_header,
            responder,
            acker,
        };
        self.dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterOneshotResponder(cmd))?;
        acker_rx.await?;
        Ok(responder_rx)
    }
    async fn close_handshake(&self) -> Result<()> {
        let responder_rx = self.register_responder(CloseChannelOk::header()).await?;
        synchronous_request!(
            self.outgoing_tx.unchecked(),
            (self.channel_id, CloseChannel::default().into_frame()),
            responder_rx,
            Frame::CloseChannelOk,
            Error::ChannelCloseError
        )?;
        // deregister channel resource from connection handler,
        // so that dispatcher will exit automatically.
        let cmd = ConnManagementCommand::DeregisterChannelResource(self.channel_id);
        self.conn_mgmt_tx.send(cmd).await?;
        Ok(())
    }
}

/////////////////////////////////////////////////////////////////////////////
impl Channel {
    /// Returns a new `Channel` instance.
    ///
    /// This does not open the channel. It is used internally by [`Connection::open_channel`].
    ///
    /// [`Connection::open_channel`]: ../connection/struct.Connection.html#method.open_channel
    pub(in crate::api) fn new(
        is_open: AtomicBool,
        connection: Connection,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::UnboundedSender<DispatcherManagementCommand>,
    ) -> Self {
        let shared = Arc::new(SharedChannelInner::new(
            is_open,
            channel_id,
            outgoing_tx,
            conn_mgmt_tx,
            dispatcher_mgmt_tx,
        ));
        let guard = Some(Arc::new(DropGuard(shared.clone())));
        Self {
            _guard: guard,
            connection,
            shared,
        }
    }

    /// Register callbacks for asynchronous message for the channel.
    ///
    /// User should always register callbacks. See [`callbacks`] documentation.
    ///
    /// # Errors
    ///
    /// Returns error if fail to send registration command.
    /// If returns [`Err`], user can try again until registration succeed.
    ///
    /// [`callbacks`]: ../callbacks/index.html
    pub async fn register_callback<F>(&self, callback: F) -> Result<()>
    where
        F: ChannelCallback + Send + 'static,
    {
        let cmd = RegisterChannelCallback {
            callback: Box::new(callback),
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterChannelCallback(cmd))?;
        Ok(())
    }

    /// Register oneshot responder for single message.
    ///
    /// Used for synchronous request/response protocol.
    async fn register_responder(
        &self,
        method_header: &'static MethodHeader,
    ) -> Result<oneshot::Receiver<IncomingMessage>> {
        let (responder, responder_rx) = oneshot::channel();
        let (acker, acker_rx) = oneshot::channel();
        let cmd = RegisterOneshotResponder {
            method_header,
            responder,
            acker,
        };
        self.shared
            .dispatcher_mgmt_tx
            .send(DispatcherManagementCommand::RegisterOneshotResponder(cmd))?;
        acker_rx.await?;
        Ok(responder_rx)
    }

    pub fn channel_id(&self) -> AmqpChannelId {
        self.shared.channel_id
    }
    pub fn connection_name(&self) -> &str {
        self.connection.connection_name()
    }
    pub fn is_connection_open(&self) -> bool {
        self.connection.is_open()
    }
    /// Returns `true` if channel is open.
    pub fn is_open(&self) -> bool {
        self.shared.is_open.load(Ordering::Relaxed)
    }
    pub(crate) async fn set_is_open(&self, is_open: bool) {
        let _admission = self.shared.outgoing_tx.admission.lock().await;
        self.shared.is_open.store(is_open, Ordering::Relaxed);
    }

    /// Asks the server to pause or restart the flow of content data.
    ///
    /// Ask to start the flow if input `active` = `true`, otherwise to pause.
    /// Also see [AMQP_0-9-1 Reference](https://github.com/rabbitmq/amqp-0.9.1-spec/blob/main/docs/amqp-0-9-1-reference.md#channel.flow).
    ///
    /// Returns `true` means the server will start/continue the flow, otherwise it will not.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    pub async fn flow(&self, active: bool) -> Result<bool> {
        let responder_rx = self.register_responder(FlowOk::header()).await?;
        let flow_ok = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, Flow::new(active).into_frame()),
            responder_rx,
            Frame::FlowOk,
            Error::ChannelUseError
        )?;
        Ok(flow_ok.active)
    }

    /// Ask the server to close the channel.
    ///
    /// To gracefully shutdown the channel, recommended to `close` the
    /// channel explicitly instead of relying on `drop`.
    ///
    /// This method consume the channel, so even it may return error,
    /// channel will anyway be dropped.
    ///
    /// # Errors
    ///
    /// Returns error if any failure in communication with server.
    /// Fail to close the channel may result in `channel leak` in server.
    pub async fn close(self) -> Result<()> {
        // if connection closed, no need to close channel
        if self.is_connection_open() {
            // check if channel is open
            if self.shared.outgoing_tx.mark_closed().await {
                #[cfg(feature = "traces")]
                info!("close channel {}", self);
                self.shared.close_handshake().await?;
            }
        }
        Ok(())
    }

    pub(crate) fn clone_as_secondary(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            connection: self.connection.clone_no_drop_guard(),
            _guard: None,
        }
    }
}

impl Drop for DropGuard {
    /// When drops, try to gracefully shutdown the channel if it is still open.
    /// It is not guaranteed to succeed in a clean way because the connection
    /// may already be closed.
    ///
    /// User is recommended to explictly call the [`close`] method.
    ///
    /// [`close`]: struct.Channel.html#method.close
    fn drop(&mut self) {
        if self.0.is_open.load(Ordering::Relaxed) {
            #[cfg(feature = "traces")]
            trace!("drop channel {}", self.0.channel_id);

            let inner = self.0.clone();
            tokio::spawn(async move {
                if !inner.outgoing_tx.mark_closed().await {
                    return;
                }
                #[cfg(feature = "traces")]
                info!("try to close channel {} at drop", inner.channel_id);
                if let Err(err) = inner.close_handshake().await {
                    // Compliance: A peer that detects a socket closure without having received a Channel.Close-Ok
                    // handshake method SHOULD log the error.
                    #[cfg(feature = "traces")]
                    error!(
                        "failed to gracefully close channel {} at drop, cause: '{}'",
                        inner.channel_id, err,
                    );
                } else {
                    #[cfg(feature = "traces")]
                    info!("channel {} is closed OK after drop", inner.channel_id);
                }
            });
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}] of connection {}",
            self.channel_id(),
            if self.is_open() { "open" } else { "closed" },
            self.connection,
        )
    }
}
///////////////////////////////////////////////////////////////////////////////
impl SharedChannelInner {
    fn new(
        is_open: AtomicBool,
        channel_id: AmqpChannelId,
        outgoing_tx: mpsc::Sender<OutgoingMessage>,
        conn_mgmt_tx: mpsc::Sender<ConnManagementCommand>,
        dispatcher_mgmt_tx: mpsc::UnboundedSender<DispatcherManagementCommand>,
    ) -> Self {
        let is_open = Arc::new(is_open);
        Self {
            outgoing_tx: ChannelOutgoingTx {
                admission: Mutex::new(()),
                is_open: is_open.clone(),
                tx: outgoing_tx,
            },
            is_open,
            channel_id,
            conn_mgmt_tx,
            dispatcher_mgmt_tx,
        }
    }
}

#[cfg(test)]
mod send_admission_tests {
    use super::*;
    use std::future::{poll_fn, Future};
    use std::task::Poll;

    fn sender() -> (Arc<ChannelOutgoingTx>, mpsc::Receiver<OutgoingMessage>) {
        let (tx, rx) = mpsc::channel(1);
        (
            Arc::new(ChannelOutgoingTx {
                admission: Mutex::new(()),
                is_open: Arc::new(AtomicBool::new(true)),
                tx,
            }),
            rx,
        )
    }

    fn frame() -> OutgoingMessage {
        (1, Flow::new(true).into_frame())
    }

    #[tokio::test]
    async fn async_send_cannot_be_overtaken_by_close() {
        let (sender, mut rx) = sender();
        sender.unchecked().send(frame()).await.unwrap();
        let send = sender.send(frame());
        tokio::pin!(send);
        assert!(poll_fn(|cx| Poll::Ready(send.as_mut().poll(cx).is_pending())).await);

        // The send has checked open state and is waiting for queue capacity.
        // Closure must wait for that admission to finish before acknowledging
        // the close and allowing the connection to recycle this channel ID.
        let close = sender.mark_closed();
        tokio::pin!(close);
        assert!(poll_fn(|cx| Poll::Ready(close.as_mut().poll(cx).is_pending())).await);
        assert!(sender.is_open.load(Ordering::Relaxed));

        rx.recv().await.unwrap();
        send.await.unwrap();
        assert!(close.await);
        rx.recv().await.unwrap();
        assert!(sender.send(frame()).await.is_err());
        assert!(rx.try_recv().is_err());

        // The protocol acknowledgement can still be enqueued after closure.
        sender
            .unchecked()
            .send((1, CloseChannelOk.into_frame()))
            .await
            .unwrap();
        assert!(matches!(
            rx.recv().await.unwrap().1,
            Frame::CloseChannelOk(_, _)
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocking_send_cannot_be_overtaken_by_close() {
        let (sender, mut rx) = sender();
        sender.unchecked().send(frame()).await.unwrap();
        let blocking_sender = sender.clone();
        let send = tokio::task::spawn_blocking(move || blocking_sender.blocking_send(frame()));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while sender.admission.try_lock().is_ok() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("blocking send must acquire admission");

        let close = sender.mark_closed();
        tokio::pin!(close);
        assert!(poll_fn(|cx| Poll::Ready(close.as_mut().poll(cx).is_pending())).await);
        rx.recv().await.unwrap();
        send.await.unwrap().unwrap();
        assert!(close.await);
        rx.recv().await.unwrap();

        let blocking_sender = sender.clone();
        assert!(
            tokio::task::spawn_blocking(move || blocking_sender.blocking_send(frame()))
                .await
                .unwrap()
                .is_err()
        );
        assert!(rx.try_recv().is_err());
    }
}

#[cfg(test)]
mod tests {
    use tokio::time;

    use crate::{
        channel::Channel,
        connection::{Connection, OpenConnectionArguments},
        test_utils::setup_logging,
    };
    use std::marker::PhantomData;

    #[ignore = "https://github.com/gftea/amqprs/issues/69"]
    #[tokio::test]
    async fn test_channel_cloneable() {
        // default: `IS_CLONEABLE = false` for all types
        trait NotCloneable {
            const IS_CLONEABLE: bool = false;
        }
        impl<T> NotCloneable for T {}

        // For all cloneable type `T`, Wrapper<T> is cloneable.
        // otherwise, it fallbacks to value from trait `NotCloneable`
        struct Wrapper<T>(PhantomData<T>);
        #[allow(dead_code)]
        impl<T: Clone> Wrapper<T> {
            const IS_CLONEABLE: bool = true;
        }
        // Prevent clippy to report assertion on const value.
        assert_eq!(<Wrapper<Channel>>::IS_CLONEABLE.to_string(), "true");
    }

    #[tokio::test]
    async fn test_channel_clone_and_drop() {
        // open one channel, clone it, and drop both, check
        setup_logging();

        // test close on drop
        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");

        let conn = Connection::open(&args).await.unwrap();
        {
            let ch1 = conn.open_channel(Some(1)).await.unwrap();
            let ch2 = ch1.clone();
            let h = tokio::spawn(async move {
                time::sleep(time::Duration::from_millis(10)).await;
                assert!(ch1.is_open());
            });
            h.await.unwrap();
            time::sleep(time::Duration::from_millis(10)).await;
            assert!(ch2.is_open());
        }
        time::sleep(time::Duration::from_millis(50)).await;
        conn.close().await.unwrap();
    }
}

/////////////////////////////////////////////////////////////////////////////
mod dispatcher;
pub(crate) use dispatcher::*;

mod basic;
mod confim;
mod exchange;
mod queue;
mod tx;

// public APIs
pub use basic::*;
pub use confim::*;
pub use exchange::*;
pub use queue::*;
#[allow(unused_imports)] // clippy false positive
pub use tx::*;
