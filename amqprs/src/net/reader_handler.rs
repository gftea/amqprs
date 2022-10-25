use std::collections::BTreeMap;

use amqp_serde::types::{AmqpChannelId, ShortStr, ShortUint};
use tokio::sync::{
    broadcast,
    mpsc::{Receiver, Sender},
};

use crate::frame::{Close, CloseChannel, CloseChannelOk, CloseOk, Frame, CONN_CTRL_CHANNEL};

use super::{
    channel_id_repo::ChannelIdRepository, BufReader, ConsumerMessage, Error, IncomingMessage,
    ManagementCommand, OutgoingMessage,
};

/////////////////////////////////////////////////////////////////////////////
struct ChannelManager {
    channel_id_repo: ChannelIdRepository,

    /// sender half to forward incoming message to AMQ Connection/Channel
    responders: BTreeMap<AmqpChannelId, Sender<IncomingMessage>>,

    /// registery of sender half of channel consumers
    consumers: BTreeMap<AmqpChannelId, BTreeMap<ConsumerTag, Sender<ConsumerMessage>>>,
}

impl ChannelManager {
    fn new(channel_max: ShortUint) -> Self {
        Self {
            channel_id_repo: ChannelIdRepository::new(channel_max),
            responders: BTreeMap::new(),
            consumers: BTreeMap::new(),
        }
    }

    fn insert_responder(
        &mut self,
        channel_id: Option<AmqpChannelId>,
        responder: Sender<IncomingMessage>,
    ) -> Option<AmqpChannelId> {
        match channel_id {
            Some(id) => {
                if self.channel_id_repo.reserve(&id) {
                    match self.responders.insert(id, responder) {
                        Some(old) => unreachable!("Implementation error"),
                        None => Some(id),
                    }
                } else {
                    None
                }
            }
            None => {
                let id = self.channel_id_repo.allocate();
                match self.responders.insert(id, responder) {
                    Some(old) => unreachable!("Implementation error"),
                    None => Some(id),
                }
            }
        }
    }
    fn get_responder(&self, channel_id: &AmqpChannelId) -> Option<&Sender<IncomingMessage>> {
        self.responders.get(channel_id)
    }
    fn remove_responder(&mut self, channel_id: &AmqpChannelId) -> Option<Sender<IncomingMessage>> {
        assert_eq!(
            true,
            self.channel_id_repo.release(channel_id),
            "Implementation error"
        );
        self.responders.remove(channel_id)
    }
}

/////////////////////////////////////////////////////////////////////////////
type ConsumerTag = String;
pub(super) struct ReaderHandler {
    stream: BufReader,

    /// sender half to forward outgoing message to `WriterHandler`
    outgoing_forwarder: Sender<OutgoingMessage>,

    /// receiver half to receive management command from AMQ Connection/Channel
    mgmt: Receiver<ManagementCommand>,

    channel_manager: ChannelManager,

    /// Notify WriterHandler to shutdown.
    /// If reader handler exit first, it will notify writer handler to shutdown.
    /// If writer handler exit first, socket connection will be shutdown because the writer half drop,
    /// so socket read will return, and reader handler can detect connection shutdown without separate signal.
    #[allow(dead_code /* notify shutdown just by dropping the instance */)]
    shutdown_notifier: broadcast::Sender<()>,
}

impl ReaderHandler {
    pub fn new(
        stream: BufReader,
        forwarder: Sender<OutgoingMessage>,
        mgmt: Receiver<ManagementCommand>,
        channel_max: ShortUint,
        shutdown_notifier: broadcast::Sender<()>,
    ) -> Self {
        Self {
            stream,
            outgoing_forwarder: forwarder,
            mgmt,
            channel_manager: ChannelManager::new(channel_max),
            shutdown_notifier,
        }
    }

    async fn handle_close(&self, close: &Close) -> Result<(), Error> {
        self.outgoing_forwarder
            .send((CONN_CTRL_CHANNEL, CloseOk::default().into_frame()))
            .await?;

        Ok(())
    }
    async fn handle_close_ok(
        &mut self,
        channel_id: AmqpChannelId,
        frame: Frame,
    ) -> Result<(), Error> {
        let responder = self
            .channel_manager
            .get_responder(&channel_id)
            .ok_or_else(|| {
                Error::InternalChannelError(format!("responder not found for CloseOk"))
            })?;

        // if receiver half has drop, which means client no longer care about the message,
        // so always returns OK and ignore SendError
        responder
            .send(IncomingMessage::Ok(frame))
            .await
            .or_else(|_| Ok(()))
    }

    async fn handle_close_channel(
        &mut self,
        channel_id: AmqpChannelId,
        close_channel: &CloseChannel,
    ) -> Result<(), Error> {
        // first, respond server we have received it
        self.outgoing_forwarder
            .send((channel_id, CloseChannelOk::default().into_frame()))
            .await?;

        // TODO: remove consumers

        // remove responder, and forward error indicated by server to client
        let responder = self
            .channel_manager
            .remove_responder(&channel_id)
            .ok_or_else(|| {
                Error::InternalChannelError(format!("responder not found for: {:?}", close_channel))
            })?;

        // respond as Exception message
        responder
            .send(IncomingMessage::Exception(
                close_channel.reply_code,
                close_channel.reply_text.clone().into(),
            ))
            .await?;
        Ok(())
    }
    async fn handle_close_channel_ok(
        &mut self,
        channel_id: AmqpChannelId,
        frame: Frame,
    ) -> Result<(), Error> {
        // remove responder
        let responder = self
            .channel_manager
            .remove_responder(&channel_id)
            .ok_or_else(|| {
                Error::InternalChannelError(format!(
                    "responder not found for CloseChannelOk, ignore it"
                ))
            })?;
        // TODO: remove consumers

        // if receiver half has drop, which means client no longer care about the message,
        // so always returns OK and ignore SendError
        responder
            .send(IncomingMessage::Ok(frame))
            .await
            .or_else(|_| Ok(()))
    }

    /// If OK, user can continue to handle frame
    /// If NOK, user should stop consuming frame
    /// TODO: implement as Iterator, then user do not need to care about the error
    async fn handle_frame(&mut self, channel_id: AmqpChannelId, frame: Frame) -> Result<(), Error> {
        match &frame {
            // TODO: handle Blocked and Unblocked from server
            Frame::Blocked(..) => todo!(),
            Frame::Unblocked(..) => todo!(),

            // Server request to close connection
            Frame::Close(_, close) => {
                assert_eq!(CONN_CTRL_CHANNEL, channel_id, "must be from channel 0");
                self.handle_close(close).await?;
                // server close connection due to error
                Err(Error::AMQPError(format!(
                    "{}: {}, casue: {}:{}",
                    close.reply_code,
                    <ShortStr as Into<String>>::into(close.reply_text.clone()),
                    close.class_id,
                    close.method_id
                )))
            }
            // Close connection response from server
            Frame::CloseOk(_, _) => {
                assert_eq!(CONN_CTRL_CHANNEL, channel_id, "must be from channel 0");
                self.handle_close_ok(channel_id, frame).await?;
                // client close connection ok
                Err(Error::PeerShutdown)
            }
            // Server request to close channel
            Frame::CloseChannel(_, close_channel) => {
                if let Err(err) = self.handle_close_channel(channel_id, close_channel).await {
                    println!("error when handling CloseChannel {}", err);
                }
                Ok(())
            }
            // Close channel response from server
            Frame::CloseChannelOk(..) => {
                if let Err(err) = self.handle_close_channel_ok(channel_id, frame).await {
                    println!("error when handling CloseChannelOk {}", err);
                }
                Ok(())
            }
            Frame::HeartBeat(_) => {
                println!("handle heartbeat...");
                Ok(())
            }
            _ => {
                // respond to synchronous request
                match self.channel_manager.get_responder(&channel_id) {
                    Some(responder) => {
                        if let Err(err) = responder.send(IncomingMessage::Ok(frame)).await {
                            println!(
                                "error when forwarding the incoming message from channel: {}",
                                channel_id
                            );
                        }
                    }
                    None => println!("no responder to route message {}", frame),
                }

                Ok(())
            }
        }
    }

    pub async fn run_until_shutdown(mut self) {
        loop {
            tokio::select! {
                Some(cmd) = self.mgmt.recv() => {
                    match cmd {
                        ManagementCommand::RegisterResponder(msg) => {
                            msg.acker.send(
                                self.channel_manager.insert_responder(msg.channel_id, msg.responder)
                            ).unwrap();
                        },
                        ManagementCommand::RegisterConsumer(_) => todo!(),
                    }
                }
                Ok((channel_id, frame)) = self.stream.read_frame() => {
                    if let Err(err) = self.handle_frame(channel_id, frame).await {
                        println!("shutdown connection, cause: {} ", err);
                        break;
                    }
                }
                else => {
                    break;
                }
            }
        }

        // `self` will drop, so the `self.shutdown_notifier`
        // all tasks which have `subscribed` to `shutdown_notifier` will be notified
        println!("shutdown reader handler!");
    }
}
