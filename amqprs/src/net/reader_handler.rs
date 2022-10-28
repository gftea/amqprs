use std::{borrow::Borrow, collections::BTreeMap, str::from_utf8};

use amqp_serde::types::{AmqpChannelId, ShortStr, ShortUint};
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver, Sender},
};

use crate::{
    api::consumer::Consumer,
    frame::{
        Ack, BasicPropertities, Close, CloseChannel, CloseChannelOk, CloseOk, Deliver, Frame,
        CONN_DEFAULT_CHANNEL,
    },
};

use super::{
    channel_id_repo::ChannelIdRepository, BufReader, ChannelResource, Error, IncomingMessage,
    ManagementCommand, OutgoingMessage,
};

/////////////////////////////////////////////////////////////////////////////
struct ChannelManager {
    /// channel id allocator and manager
    channel_id_repo: ChannelIdRepository,

    /// channel resource registery store
    resource: BTreeMap<AmqpChannelId, ChannelResource>,
}

impl ChannelManager {
    fn new(channel_max: ShortUint) -> Self {
        Self {
            channel_id_repo: ChannelIdRepository::new(channel_max),
            resource: BTreeMap::new(),
        }
    }

    fn insert_resource(
        &mut self,
        channel_id: Option<AmqpChannelId>,
        resource: ChannelResource,
    ) -> Option<AmqpChannelId> {
        let id = match channel_id {
            // reserve channel id as requested
            Some(id) => {
                if self.channel_id_repo.reserve(&id) {
                    match self.resource.insert(id, resource) {
                        Some(old) => unreachable!("Implementation error"),
                        None => id,
                    }
                } else {
                    // fail to reserve the id
                    return None;
                }
            }
            // allocate a channel id
            None => {
                // allocate id never fail
                let id = self.channel_id_repo.allocate();
                match self.resource.insert(id, resource) {
                    Some(old) => unreachable!("Implementation error"),
                    None => id,
                }
            }
        };

        Some(id)
    }

    fn get_responder(&self, channel_id: &AmqpChannelId) -> Option<&Sender<IncomingMessage>> {
        let responder = &self.resource.get(channel_id)?.responder;
        Some(responder)
    }

    fn get_dispatcher(&self, channel_id: &AmqpChannelId) -> Option<&Sender<Frame>> {
        self.resource.get(channel_id)?.dispatcher.as_ref()
    }
    fn remove_resource(&mut self, channel_id: &AmqpChannelId) -> Option<ChannelResource> {
        assert_eq!(
            true,
            self.channel_id_repo.release(channel_id),
            "Implementation error"
        );
        // remove responder means channel is to be  closed
        self.resource.remove(channel_id)
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
    // consumer buffer
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
    async fn spawn_channel_consumer(&self, channel_id: AmqpChannelId) -> Sender<Frame> {
        let acker = self.outgoing_forwarder.clone();
        let (tx, mut rx) = mpsc::channel::<Frame>(32);
        tokio::spawn(async move {
            loop {
                #[derive(Debug)]
                struct ConsumerMessage {
                    deliver: Option<Deliver>,
                    basic_propertities: Option<BasicPropertities>,
                    content: Option<Vec<u8>>,
                }
                let mut message = ConsumerMessage {
                    deliver: None,
                    basic_propertities: None,
                    content: None,
                };

                match rx.recv().await.unwrap() {
                    Frame::Deliver(_, deliver) => {
                        println!("consumer recv: deliver");
                        message.deliver = Some(deliver);
                    }
                    Frame::ContentHeader(header) => {
                        println!("consumer recv: content header");
                        message.basic_propertities = Some(header.basic_propertities);
                    }
                    Frame::ContentBody(body) => {
                        println!("consumer recv: content body");
                        message.content = Some(body.inner);

                        println!("<<<<< recv combined consumer message: {:?}", message);

                        let delivery_tag = message.deliver.unwrap().delivery_tag;
                        let ack = Ack {
                            delivery_tag,
                            mutiple: false,
                        };
                        acker.send((channel_id, ack.into_frame())).await.unwrap();
                        println!(">>>> ack message: {:?}", delivery_tag);
                    }
                    _ => unreachable!(),
                }
            }
        });
        tx
    }
    async fn handle_close(&self, close: &Close) -> Result<(), Error> {
        self.outgoing_forwarder
            .send((CONN_DEFAULT_CHANNEL, CloseOk::default().into_frame()))
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
        // use `try_send` instead of `send` because client may not start receiver
        // it will return immediately instead of blocking wait
        responder
            .try_send(IncomingMessage::Ok(frame))
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
            .remove_resource(&channel_id)
            .ok_or_else(|| {
                Error::InternalChannelError(format!("responder not found for: {:?}", close_channel))
            })?
            .responder;

        // respond as Exception message
        // use `try_send` instead of `send` because client may not start receiver
        // it will return immediately instead of blocking wait
        responder
            .try_send(IncomingMessage::Exception(
                close_channel.reply_code,
                close_channel.reply_text.clone().into(),
            ))
            .or_else(|_| Ok(()))
    }

    ///
    async fn handle_close_channel_ok(
        &mut self,
        channel_id: AmqpChannelId,
        frame: Frame,
    ) -> Result<(), Error> {
        // remove responder
        let responder = self
            .channel_manager
            .remove_resource(&channel_id)
            .ok_or_else(|| {
                Error::InternalChannelError(format!(
                    "responder not found for CloseChannelOk, ignore it"
                ))
            })?
            .responder;
        // TODO: remove consumers

        // if receiver half has drop, which means client no longer care about the message,
        // so always returns OK and ignore SendError
        // use `try_send` instead of `send` because client may not start receiver
        // it will return immediately instead of blocking wait
        responder
            .try_send(IncomingMessage::Ok(frame))
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
                assert_eq!(CONN_DEFAULT_CHANNEL, channel_id, "must be from channel 0");
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
                assert_eq!(CONN_DEFAULT_CHANNEL, channel_id, "must be from channel 0");
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

            // Deliver, ContentHeader, ContentBody are delivered in sequence by server.
            // Need to store the deliver frame and expect upcoming content.
            Frame::Deliver(_, _) | Frame::ContentHeader(_) | Frame::ContentBody(_) => {
                match self.channel_manager.get_dispatcher(&channel_id) {
                    Some(dispatcher) => {
                        dispatcher.send(frame).await?;
                        Ok(())
                    }
                    None => {
                        println!("no consumer registered yet, discard : {:?}", frame);
                        Ok(())
                    }
                }
            }
            // Frame::ContentHeader(header) => {
            //     match self.channel_manager.get_consumer(&channel_id) {
            //         Some(res) => {
            //             res.consumer_tx.send(frame).await?;
            //             Ok(())

            //         },
            //         None => Ok(())
            //     }
            // }
            // Frame::ContentBody(body) => {
            //     match self.channel_manager.get_consumer(&channel_id) {
            //         Some(res) => {
            //             res.consumer_tx.send(frame).await?;
            //             Ok(())

            //         },
            //         None => Ok(())
            //     }
            // }
            _ => {
                // respond to synchronous request
                match self.channel_manager.get_responder(&channel_id) {
                    Some(responder) => {
                        // use `try_send` instead of `send` because client may not start receiver
                        // it will return immediately instead of blocking wait
                        if let Err(err) = responder.try_send(IncomingMessage::Ok(frame)) {
                            println!(
                                "error when forwarding the incoming message from channel: {}, cause: {}",
                                channel_id, err
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
                biased;

                Some(cmd) = self.mgmt.recv() => {
                    match cmd {
                        ManagementCommand::RegisterChannelResource(msg) => {
                            let id = self.channel_manager.insert_resource(msg.channel_id, msg.resource);
                            msg.acker.send(id).unwrap();
                        },
                    }
                }

                res = self.stream.read_frame() => {
                    match res {
                        Ok((channel_id, frame)) => {
                            if let Err(err) = self.handle_frame(channel_id, frame).await {
                                println!("shutdown connection, cause: {} ", err);
                                break;
                            }
                        },
                        Err(err) => {
                            println!("fail to read frame, cause: {}", err);
                            break;
                        },
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
