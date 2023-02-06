use std::collections::{HashMap, VecDeque};

use tokio::{
    sync::{mpsc, oneshot},
    task::yield_now,
    time,
};

use crate::{
    api::{callbacks::ChannelCallback, channel::ReturnMessage},
    channel::GetOkMessage,
    frame::{CancelOk, CloseChannelOk, ContentBody, FlowOk, Frame, MethodHeader},
    net::IncomingMessage,
    BasicProperties, Return,
};
#[cfg(feature = "traces")]
use tracing::{debug, error, info, trace};

use super::{Channel, ConsumerMessage, DispatcherManagementCommand};

/// Assumption:
/// Depends on total number of consumers per channel, a reasonable value
/// should be selected. Assume most cases, searching expiry consumers
/// every `10` seconds does not impact performance
const CONSUMER_PURGE_INTERVAL: time::Duration = time::Duration::from_secs(10);

/// Assumption:
/// Consumer is expected to be registered right after `consume/consume-ok` handshake is done
/// which can't take longer than `5` seconds.
/// After consumer is canceled, all on-the-fly messages should be received within `5` seconds
const CONSUMER_EXPIRY_PERIOD: time::Duration = time::Duration::from_secs(5);

/// Resource for handling consumer messages.
struct ConsumerResource {
    /// FIFO buffer for a delivery = `deliver + content`.
    fifo: VecDeque<ConsumerMessage>,
    /// tx channel to forward a delivery to a consumer task.
    /// dispatcher task holds the tx half, and the consumer task holds the rx half.
    tx: Option<mpsc::UnboundedSender<ConsumerMessage>>,
    /// expiry time of fifo buffer
    expiration: Option<time::Instant>,
}

impl ConsumerResource {
    fn new() -> Self {
        Self {
            fifo: VecDeque::new(),
            tx: None,
            expiration: Some(time::Instant::now() + CONSUMER_EXPIRY_PERIOD),
        }
    }

    fn register_tx(
        &mut self,
        tx: mpsc::UnboundedSender<ConsumerMessage>,
    ) -> Option<mpsc::UnboundedSender<ConsumerMessage>> {
        // once consumer's tx half is registered, clear the expiry timer
        self.expiration.take();
        self.tx.replace(tx)
    }

    fn get_tx(&self) -> Option<&mpsc::UnboundedSender<ConsumerMessage>> {
        self.tx.as_ref()
    }

    fn get_expiration(&self) -> Option<&time::Instant> {
        self.expiration.as_ref()
    }

    fn push_message(&mut self, message: ConsumerMessage) {
        self.fifo.push_back(message);
    }

    fn pop_message(&mut self) -> Option<ConsumerMessage> {
        self.fifo.pop_front()
    }
}

enum State {
    Initial,
    Deliver,
    GetOk,
    GetEmpty,
    Return,
}

/// Dispatcher for a channel.
///
/// Each channel will spawn a dispatcher.
/// It handles channel level callbacks, incoming messages and registration commands.
/// It also dispatch messages to consumers.
pub(crate) struct ChannelDispatcher {
    channel: Channel,
    dispatcher_rx: mpsc::UnboundedReceiver<IncomingMessage>,
    dispatcher_mgmt_rx: mpsc::UnboundedReceiver<DispatcherManagementCommand>,
    consumer_resources: HashMap<String, ConsumerResource>,
    get_content_responder: Option<mpsc::UnboundedSender<IncomingMessage>>,
    responders: HashMap<&'static MethodHeader, oneshot::Sender<IncomingMessage>>,
    callback: Option<Box<dyn ChannelCallback + Send + 'static>>,
    state: State,
}
/////////////////////////////////////////////////////////////////////////////
impl ChannelDispatcher {
    pub(crate) fn new(
        channel: Channel,
        dispatcher_rx: mpsc::UnboundedReceiver<IncomingMessage>,
        dispatcher_mgmt_rx: mpsc::UnboundedReceiver<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            channel,
            dispatcher_rx,
            dispatcher_mgmt_rx,
            consumer_resources: HashMap::new(),
            get_content_responder: None,
            responders: HashMap::new(),
            callback: None,
            state: State::Initial,
        }
    }

    /// Return the consumer resource if it always exists, otherwise create new one.
    fn get_or_new_consumer_resource(&mut self, consumer_tag: &String) -> &mut ConsumerResource {
        if !self.consumer_resources.contains_key(consumer_tag) {
            let resource = ConsumerResource::new();
            self.consumer_resources
                .insert(consumer_tag.clone(), resource);
        }
        self.consumer_resources.get_mut(consumer_tag).unwrap()
    }

    /// purge expired consumer resource
    fn purge_consumer_resource(&mut self) {
        // find all resources that are expired
        let purge_keys: Vec<String> = self
            .consumer_resources
            .iter()
            .filter_map(|(k, v)| {
                if let Some(expiration) = v.get_expiration() {
                    if expiration < &time::Instant::now() {
                        return Some(k.clone());
                    }
                }
                None
            })
            .collect();

        // purge expired resources
        for key in purge_keys {
            self.consumer_resources.remove(&key);
            #[cfg(feature = "traces")]
            debug!(
                "purge stale consumer resource {} on channel {}",
                key, self.channel
            );
        }
    }
    /// Remove the consumer resource.
    ///
    /// Becuase the tx channel will drop, the consumer task will also exit.
    fn remove_consumer_resource(&mut self, consumer_tag: &String) -> Option<ConsumerResource> {
        self.consumer_resources.remove(consumer_tag)
    }

    async fn forward_deliver(&mut self, consumer_message: ConsumerMessage) {
        let consumer_tag = consumer_message
            .deliver
            .as_ref()
            .unwrap()
            .consumer_tag()
            .clone();
        let consumer = self.get_or_new_consumer_resource(&consumer_tag);
        match consumer.get_tx() {
            Some(consumer_tx) => {
                if (consumer_tx.send(consumer_message)).is_err() {
                    #[cfg(feature = "traces")]
                    error!(
                        "failed to dispatch message to consumer {} on channel {}",
                        consumer_tag, self.channel
                    );
                }
            }
            None => {
                #[cfg(feature = "traces")]
                debug!("can't find consumer {}, message is buffered", consumer_tag);
                consumer.push_message(consumer_message);
                // try to yield for expected consumer registration command,
                // it might reduceas buffering
                yield_now().await;
            }
        };
    }

    async fn handle_return(
        &mut self,
        ret: Return,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        if let Some(ref mut cb) = self.callback {
            cb.publish_return(&self.channel, ret, basic_properties, content)
                .await;
        } else {
            #[cfg(feature = "traces")]
            error!("callback not registered on channel {}", self.channel);
        }
    }
    /// Spawn dispatcher task.
    pub(in crate::api) async fn spawn(mut self) {
        tokio::spawn(async move {
            // aggregation buffer for `deliver + content` messages to a consumer
            let mut message_buffer = ConsumerMessage {
                deliver: None,
                basic_properties: None,
                content: None,
                remaining: 0,
            };
            // buffer for `return + content` messages due to publish failure.
            let mut return_buffer = ReturnMessage {
                ret: None,
                basic_properties: None,
                content: None,
                remaining: 0,
            };
            // buffer for `getok + content` messages
            let mut getok_content_buffer = GetOkMessage {
                content: None,
                remaining: 0,
            };

            #[cfg(feature = "traces")]
            trace!("starts up dispatcher task of channel {}", self.channel);

            let mut purge_timer = time::interval(CONSUMER_PURGE_INTERVAL);
            purge_timer.tick().await;
            // main loop of dispatcher
            loop {
                tokio::select! {
                    biased;

                    // the dispatcher also holds a `Channel` instance, so this
                    // should never return `None`
                    command = self.dispatcher_mgmt_rx.recv() => {
                        // handle command channel error
                        let cmd = match command {
                            None => {
                                unreachable!("dispatcher command channel closed, {}", self.channel);
                            },
                            Some(v) => v,
                        };
                        // handle command
                        match cmd {
                            DispatcherManagementCommand::RegisterContentConsumer(cmd) => {
                                #[cfg(feature="traces")]
                                info!("register consumer {}", cmd.consumer_tag);
                                let consumer = self.get_or_new_consumer_resource(&cmd.consumer_tag);
                                consumer.register_tx(cmd.consumer_tx);
                                // forward buffered messages
                                while !consumer.fifo.is_empty() {
                                    #[cfg(feature="traces")]
                                    trace!("consumer {} total buffered messages: {}", cmd.consumer_tag, consumer.fifo.len());
                                    let msg = consumer.pop_message().unwrap();
                                    if let Err(_err) = consumer.get_tx().unwrap().send(msg) {
                                        #[cfg(feature="traces")]
                                        error!("failed to forward message to consumer {}", cmd.consumer_tag);
                                    }
                                }
                            },
                            DispatcherManagementCommand::DeregisterContentConsumer(cmd) => {
                                if let Some(consumer) = self.remove_consumer_resource(&cmd.consumer_tag) {
                                    #[cfg(feature="traces")]
                                    info!("deregister consumer {}, total buffered messages: {}",
                                        cmd.consumer_tag, consumer.fifo.len()
                                    );

                                }
                            },
                            DispatcherManagementCommand::RegisterGetContentResponder(cmd) => {
                                self.get_content_responder.replace(cmd.tx);
                            }
                            DispatcherManagementCommand::RegisterOneshotResponder(cmd) => {
                                self.responders.insert(cmd.method_header, cmd.responder);
                                cmd.acker.send(()).unwrap();
                            }
                            DispatcherManagementCommand::RegisterChannelCallback(cmd) => {
                                self.callback.replace(cmd.callback);
                                #[cfg(feature="traces")]
                                debug!("callback registered on channel {}", self.channel);
                            }
                        }
                    }
                    // only one tx half held by connection handler, once the tx half dorp
                    // it will return `None`, so exit the dispatcher
                    message = self.dispatcher_rx.recv() => {
                        // handle message channel error
                        let frame = match message {
                            None => {
                                // exit
                                #[cfg(feature="traces")]
                                debug!("dispatcher mpsc channel closed, channel {}", self.channel);
                                break;
                            },
                            Some(v) => v,
                        };
                        // handle frames
                        match frame {
                            ////////////////////////////////////////////////
                            // frames for closing channel
                            // channel.close-ok response from server
                            Frame::CloseChannelOk(method_header, close_channel_ok) => {
                                self.channel.set_is_open(false);

                                match self.responders.remove(method_header) {
                                    Some(responder) => responder.send(close_channel_ok.into_frame()).unwrap(),
                                    None => unreachable!("responder must be registered for {} on channel {}",
                                    close_channel_ok.into_frame(), self.channel),
                                }
                                // exit
                                break;
                            }
                            // channel.close request from server
                            Frame::CloseChannel(_, close_channel) => {
                                // callback
                                if let Some(ref mut cb) = self.callback {
                                    if let Err(err) = cb.close(&self.channel, close_channel).await {
                                      #[cfg(feature="traces")]
                                      error!("close callback returns error on channel {}, cause: {}", self.channel, err);
                                      // exit immediately, no response to server
                                      break;
                                    };
                                } else {
                                    #[cfg(feature="traces")]
                                    error!("callback not registered on channel {}", self.channel);
                                }
                                self.channel.set_is_open(false);

                                // implictly respond OK to server
                                self.channel.shared.outgoing_tx
                                .send((self.channel.channel_id(), CloseChannelOk::default().into_frame()))
                                .await.unwrap();
                                // exit
                                break;
                            }
                            ////////////////////////////////////////////////
                            // the method frames followed by content frames
                            Frame::GetEmpty(_, get_empty) => {
                                self.state = State::GetEmpty;

                                self.get_content_responder.take()
                                .expect("get responder must be registered")
                                .send(get_empty.into_frame()).unwrap();
                            }
                            Frame::GetOk(_, get_ok) => {
                                self.state = State::GetOk;

                                self.get_content_responder.as_ref()
                                .expect("get responder must be registered")
                                .send(get_ok.into_frame()).unwrap();
                            }
                            Frame::Return(_, ret) => {
                                self.state = State::Return;
                                return_buffer.ret = Some(ret);
                            }
                            Frame::Deliver(_, deliver) => {
                                self.state = State::Deliver;
                                message_buffer.deliver = Some(deliver);
                            }
                            Frame::ContentHeader(header) => {
                                match self.state {
                                    State::Deliver => {
                                        message_buffer.remaining = header.common.body_size.try_into().unwrap();
                                        // do not wait for content body frame if content body size is zero
                                        if message_buffer.remaining == 0 {
                                            let consumer_message  = ConsumerMessage {
                                                deliver: message_buffer.deliver.take(),
                                                basic_properties: Some(header.basic_properties),
                                                content: Some(Vec::new()),
                                                remaining: 0,
                                            };
                                            self.forward_deliver(consumer_message).await;
                                        } else {
                                            message_buffer.basic_properties = Some(header.basic_properties);
                                            message_buffer.content = Some(Vec::new());
                                        }
                                    },
                                    State::GetOk => {
                                        getok_content_buffer.remaining = header.common.body_size.try_into().unwrap();

                                        let responder = self.get_content_responder.as_ref().expect("get responder must be registered");
                                        responder.send(header.into_frame()).unwrap();
                                        // do not wait for content body frame if content body size is zero
                                        if getok_content_buffer.remaining  == 0 {
                                            responder.send(ContentBody::new(Vec::new()).into_frame()).unwrap();
                                        } else {
                                            getok_content_buffer.content = Some(Vec::new());
                                        }
                                    },
                                    State::Return => {
                                        return_buffer.remaining = header.common.body_size.try_into().unwrap();

                                        if return_buffer.remaining == 0 {
                                            // do not wait for content body frame if content body size is zero
                                            self.handle_return(return_buffer.ret.take().unwrap(), header.basic_properties, Vec::new()).await;
                                        } else {
                                            return_buffer.basic_properties = Some(header.basic_properties);
                                            return_buffer.content = Some(Vec::new());
                                        }
                                    },
                                    _  => unreachable!("invalid dispatcher state"),
                                }
                            }
                            Frame::ContentBody(body) => {
                                match self.state {
                                    State::Deliver => {
                                        let mut content_buffer = message_buffer.content.take().unwrap();
                                        content_buffer.extend_from_slice(&body.inner);
                                        message_buffer.content.replace(content_buffer);
                                        // calculate remaining size of content body
                                        message_buffer.remaining = message_buffer.remaining.checked_sub(body.inner.len()).expect("should never overflow");

                                        if message_buffer.remaining == 0 {
                                            let consumer_message  = ConsumerMessage {
                                                deliver: message_buffer.deliver.take(),
                                                basic_properties: message_buffer.basic_properties.take(),
                                                content: message_buffer.content.take(),
                                                remaining: message_buffer.remaining,
                                            };
                                            self.forward_deliver(consumer_message).await;
                                        }
                                    }
                                    State::GetOk => {
                                        let mut content_buffer = getok_content_buffer.content.take().unwrap();
                                        content_buffer.extend_from_slice(&body.inner);
                                        getok_content_buffer.content.replace(content_buffer);
                                        getok_content_buffer.remaining = getok_content_buffer.remaining.checked_sub(body.inner.len()).expect("should never overflow");
                                        if getok_content_buffer.remaining == 0 {
                                            let content = getok_content_buffer.content.take().unwrap();
                                            self.get_content_responder.take()
                                            .expect("get responder must be registered")
                                            .send(ContentBody::new(content).into_frame()).unwrap();
                                        }
                                    },
                                    State::Return => {
                                        let mut content_buffer = return_buffer.content.take().unwrap();
                                        content_buffer.extend_from_slice(&body.inner);
                                        return_buffer.content.replace(content_buffer);
                                        return_buffer.remaining = return_buffer.remaining.checked_sub(body.inner.len()).expect("should never overflow");

                                        if return_buffer.remaining == 0 {
                                            self.handle_return(
                                                return_buffer.ret.take().unwrap(),
                                                return_buffer.basic_properties.take().unwrap(),
                                                return_buffer.content.take().unwrap()).await;
                                        }
                                    },
                                    State::Initial | State::GetEmpty  => unreachable!("invalid dispatcher state on channel {}", self.channel),
                                }
                            }

                            ////////////////////////////////////////////////
                            // synchronous response frames
                            Frame::FlowOk(method_header, _)
                            // | Frame::RequestOk(method_header, _) // Deprecated
                            | Frame::DeclareOk(method_header, _)
                            | Frame::DeleteOk(method_header, _)
                            | Frame::BindOk(method_header, _)
                            | Frame::UnbindOk(method_header, _)
                            | Frame::DeclareQueueOk(method_header, _)
                            | Frame::BindQueueOk(method_header, _)
                            | Frame::PurgeQueueOk(method_header, _)
                            | Frame::DeleteQueueOk(method_header, _)
                            | Frame::UnbindQueueOk(method_header, _)
                            | Frame::QosOk(method_header, _)
                            | Frame::ConsumeOk(method_header, _)
                            | Frame::CancelOk(method_header, _)
                            | Frame::RecoverOk(method_header, _)
                            | Frame::SelectOk(method_header, _)
                            | Frame::TxSelectOk(method_header, _)
                            | Frame::TxCommitOk(method_header, _)
                            | Frame::TxRollbackOk(method_header, _) => {
                                // handle synchronous response
                                match self.responders.remove(method_header)
                                {
                                    Some(responder) => {
                                        if let Err(response) = responder.send(frame) {
                                            #[cfg(feature="traces")]
                                            error!(
                                                "failed to dispatch {} to channel {}",
                                                response, self.channel
                                            );
                                        }
                                    }
                                    None => unreachable!(
                                        "responder must be registered for {} on channel {}",
                                        frame, self.channel
                                    ),
                                }
                            }
                            //////////////////////////////////////////////////////////
                            // asynchronous request frames
                            Frame::Flow(_, flow) => {
                                // callback
                                if let Some(ref mut cb) = self.callback {
                                    match cb.flow(&self.channel, flow.active).await {
                                      Err(err) => {
                                        #[cfg(feature="traces")]
                                        error!("flow callback error on channel {}, cause: '{}'.", self.channel, err);
                                      }
                                      Ok(active) => {
                                         // respond to server that we have handled the request
                                         self.channel.shared.outgoing_tx
                                         .send((self.channel.channel_id(), FlowOk::new(active).into_frame()))
                                         .await.unwrap();
                                      }
                                    };
                                } else {
                                    #[cfg(feature="traces")]
                                    error!("callback not registered on channel {}", self.channel);
                                }

                            }
                            Frame::Cancel(_, cancel) => {
                                // callback
                                if let Some(ref mut cb) = self.callback {
                                    let consumer_tag = cancel.consumer_tag().clone();
                                    let no_wait = cancel.no_wait();
                                    match cb.cancel(&self.channel, cancel).await {
                                      Err(err) => {
                                        #[cfg(feature="traces")]
                                        error!("cancel callback error on channel {}, cause: '{}'.", self.channel, err);
                                      }
                                      Ok(_) => {
                                        self.remove_consumer_resource(&consumer_tag);

                                        // respond to server that we have handled the request
                                        if !no_wait  {
                                            self.channel.shared.outgoing_tx
                                            .send((self.channel.channel_id(), CancelOk::new(consumer_tag.try_into().unwrap()).into_frame()))
                                            .await.unwrap();
                                        }
                                      }
                                    };
                                } else {
                                    #[cfg(feature="traces")]
                                    error!("callback not registered on channel {}", self.channel);
                                }
                            }
                            // in confirmed mode
                            Frame::Ack(_, ack) => {
                                if let Some(ref mut cb) = self.callback {
                                    cb.publish_ack(&self.channel, ack).await;
                                } else {
                                    #[cfg(feature="traces")]
                                    error!("callback not registered on channel {}", self.channel);
                                }
                            }
                            Frame::Nack(_, nack) => {
                                if let Some(ref mut cb) = self.callback {
                                    cb.publish_nack(&self.channel, nack).await;
                                } else {
                                    #[cfg(feature="traces")]
                                    error!("callback not registered on channel {}", self.channel);
                                }                            }
                            _ => unreachable!("dispatcher of channel {} receive unexpected frame {}", self.channel, frame),
                        }
                    }
                    // purge stale consumer resource
                    _ = purge_timer.tick() => {
                        self.purge_consumer_resource();
                    }
                    else => {
                        break;
                    }

                }
            }
            #[cfg(feature = "traces")]
            info!("exit dispatcher of channel {}", self.channel);
        });
    }
}

#[cfg(test)]
mod tests {
    use tokio::time;

    use crate::{
        channel::{
            BasicCancelArguments, BasicConsumeArguments, BasicPublishArguments, QueueBindArguments,
            QueueDeclareArguments,
        },
        connection::{Connection, OpenConnectionArguments},
        consumer::DefaultConsumer,
        test_utils::setup_logging,
        BasicProperties,
    };

    use super::{CONSUMER_EXPIRY_PERIOD, CONSUMER_PURGE_INTERVAL};

    #[tokio::test]
    async fn test_purge_consumer_resource() {
        setup_logging();

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "bitnami");
        let connection = Connection::open(&args).await.unwrap();

        let exchange_name = "amq.topic";
        let routing_key = "test.purge.consumer";

        let consumer_channel = connection.open_channel(None).await.unwrap();
        let (queue_name, _, _) = consumer_channel
            .queue_declare(QueueDeclareArguments::default())
            .await
            .unwrap()
            .unwrap();
        consumer_channel
            .queue_bind(QueueBindArguments::new(
                &queue_name,
                exchange_name,
                routing_key,
            ))
            .await
            .unwrap();

        // publish messages first so that messages
        // are redelivered immediately once we start consumer
        let pub_channel = connection.open_channel(None).await.unwrap();

        for _ in 0..100 {
            pub_channel
                .basic_publish(
                    BasicProperties::default(),
                    String::from("stale message").into_bytes(),
                    BasicPublishArguments::new(exchange_name, routing_key),
                )
                .await
                .unwrap();
        }
        // wait for publish done
        time::sleep(time::Duration::from_secs(1)).await;

        // start consumer with no_wait = true
        let consumer_tag = consumer_channel
            .basic_consume(
                DefaultConsumer::new(false),
                BasicConsumeArguments::new(&queue_name, "purge-tester")
                    .no_wait(true)
                    .finish(),
            )
            .await
            .unwrap();

        // immediately cancel consumer with no_wait = true
        consumer_channel
            .basic_cancel(
                BasicCancelArguments::new(&consumer_tag)
                    .no_wait(true)
                    .finish(),
            )
            .await
            .unwrap();

        // the consumer resource should be purged within `CONSUMER_PURGE_INTERVAL + CONSUMER_EXPIRY_PERIOD`
        time::sleep(CONSUMER_PURGE_INTERVAL + CONSUMER_EXPIRY_PERIOD).await;
    }
}
