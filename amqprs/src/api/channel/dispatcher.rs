use std::collections::{HashMap, VecDeque};

use tokio::{
    sync::{mpsc, oneshot},
    task::yield_now,
};

use crate::{
    api::{callbacks::ChannelCallback, channel::ReturnMessage},
    frame::{CancelOk, CloseChannelOk, FlowOk, Frame, MethodHeader},
    net::IncomingMessage,
};
use tracing::{debug, error, info, trace};

use super::{Channel, ConsumerMessage, DispatcherManagementCommand};

/// Resource for handling consumer messages.
struct ConsumerResource {
    /// FIFO buffer for a delivery = `deliver + content`.
    fifo: VecDeque<ConsumerMessage>,
    /// tx channel to forward a delivery to a consumer task.
    /// dispatcher task holds the tx half, and the consumer task holds the rx half.
    tx: Option<mpsc::Sender<ConsumerMessage>>,
}

impl ConsumerResource {
    fn new() -> Self {
        Self {
            fifo: VecDeque::new(),
            tx: None,
        }
    }

    fn register_tx(
        &mut self,
        tx: mpsc::Sender<ConsumerMessage>,
    ) -> Option<mpsc::Sender<ConsumerMessage>> {
        self.tx.replace(tx)
    }

    fn get_tx(&self) -> Option<&mpsc::Sender<ConsumerMessage>> {
        self.tx.as_ref()
    }

    fn push(&mut self, message: ConsumerMessage) {
        self.fifo.push_back(message);
    }

    fn pop(&mut self) -> Option<ConsumerMessage> {
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
    dispatcher_rx: mpsc::Receiver<IncomingMessage>,
    dispatcher_mgmt_rx: mpsc::Receiver<DispatcherManagementCommand>,
    consumers: HashMap<String, ConsumerResource>,
    get_content_responder: Option<mpsc::Sender<IncomingMessage>>,
    responders: HashMap<&'static MethodHeader, oneshot::Sender<IncomingMessage>>,
    callback: Option<Box<dyn ChannelCallback + Send + 'static>>,
    state: State,
}
/////////////////////////////////////////////////////////////////////////////
impl ChannelDispatcher {
    pub(crate) fn new(
        channel: Channel,
        dispatcher_rx: mpsc::Receiver<IncomingMessage>,
        dispatcher_mgmt_rx: mpsc::Receiver<DispatcherManagementCommand>,
    ) -> Self {
        Self {
            channel,
            dispatcher_rx,
            dispatcher_mgmt_rx,
            consumers: HashMap::new(),
            get_content_responder: None,
            responders: HashMap::new(),
            callback: None,
            state: State::Initial,
        }
    }

    /// Return the consumer resource if it always exists, otherwise create new one.
    fn get_or_new_consumer(&mut self, consumer_tag: &String) -> &mut ConsumerResource {
        if !self.consumers.contains_key(consumer_tag) {
            let resource = ConsumerResource::new();
            self.consumers.insert(consumer_tag.clone(), resource);
        }
        self.consumers.get_mut(consumer_tag).unwrap()
    }

    /// Remove the consumer resource.
    ///
    /// Becuase the tx channel will drop, the consumer task will also exit.
    fn remove_consumer(&mut self, consumer_tag: &String) -> Option<ConsumerResource> {
        self.consumers.remove(consumer_tag)
    }

    /// Spawn dispatcher task.
    pub(in crate::api) async fn spawn(mut self) {
        tokio::spawn(async move {
            // aggregation buffer for `deliver + content` messages to a consumer
            let mut message_buffer = ConsumerMessage {
                deliver: None,
                basic_properties: None,
                content: None,
            };
            // buffer for `return + content` messages due to publish failure.
            let mut return_buffer = ReturnMessage {
                ret: None,
                basic_properties: None,
            };
            trace!("starts up dispatcher task of channel {}", self.channel);
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
                                info!("register consumer {}", cmd.consumer_tag);
                                let consumer = self.get_or_new_consumer(&cmd.consumer_tag);
                                consumer.register_tx(cmd.consumer_tx);
                                // forward buffered messages
                                while !consumer.fifo.is_empty() {
                                    trace!("consumer {} total buffered messages: {}", cmd.consumer_tag, consumer.fifo.len());
                                    let msg = consumer.pop().unwrap();
                                    if let Err(_err) = consumer.get_tx().unwrap().send(msg).await {
                                        error!("failed to forward message to consumer {}", cmd.consumer_tag);
                                    }
                                }
                            },
                            DispatcherManagementCommand::DeregisterContentConsumer(cmd) => {
                                if let Some(consumer) = self.remove_consumer(&cmd.consumer_tag) {
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
                                debug!("dispatcher message channel closed, {}", self.channel);
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
                                      error!("close callback returns error on channel {}, cause: {}", self.channel, err);
                                      // exit immediately, no response to server
                                      break;
                                    };
                                } else {
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
                                .send(get_empty.into_frame()).await.unwrap();
                            }
                            Frame::GetOk(_, get_ok) => {
                                self.state = State::GetOk;

                                self.get_content_responder.as_ref()
                                .expect("get responder must be registered")
                                .send(get_ok.into_frame()).await.unwrap();
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
                                    State::Deliver => message_buffer.basic_properties = Some(header.basic_properties),
                                    State::GetOk => {
                                        self.get_content_responder.as_ref()
                                        .expect("get responder must be registered")
                                        .send(header.into_frame()).await.unwrap();
                                    },
                                    State::Return => return_buffer.basic_properties = Some(header.basic_properties),
                                    _  => unreachable!("invalid dispatcher state"),
                                }
                            }
                            Frame::ContentBody(body) => {
                                match self.state {
                                    State::Deliver => {
                                        message_buffer.content = Some(body.inner);

                                        let consumer_tag = message_buffer.deliver.as_ref().unwrap().consumer_tag().clone();
                                        let consumer_message  = ConsumerMessage {
                                            deliver: message_buffer.deliver.take(),
                                            basic_properties: message_buffer.basic_properties.take(),
                                            content: message_buffer.content.take(),
                                        };
                                        let consumer = self.get_or_new_consumer(&consumer_tag);
                                        match consumer.get_tx() {
                                            Some(consumer_tx) => {
                                                if let Err(_) = consumer_tx.send(consumer_message).await {
                                                    error!("failed to dispatch message to consumer {} on channel {}",
                                                    consumer_tag, self.channel);
                                                }
                                            },
                                            None => {
                                                info!("can't find consumer {}, buffering message", consumer_tag);
                                                consumer.push(consumer_message);
                                                // FIXME: try to yield for registering consumer
                                                //      not sure if it is necessary
                                                yield_now().await;
                                            },
                                        };
                                    }
                                    State::GetOk => {
                                        self.get_content_responder.take()
                                        .expect("get responder must be registered")
                                        .send(body.into_frame()).await.unwrap();
                                    },
                                    State::Return => {
                                        if let Some(ref mut cb) = self.callback {
                                            cb.publish_return(
                                                &self.channel,
                                                return_buffer.ret.take().unwrap(),
                                                return_buffer.basic_properties.take().unwrap(),
                                                body.inner
                                            ).await ;
                                        } else {
                                            error!("callback not registered on channel {}", self.channel);
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
                                        error!("cancel callback error on channel {}, cause: '{}'.", self.channel, err);
                                      }
                                      Ok(_) => {
                                        self.remove_consumer(&consumer_tag);

                                        // respond to server that we have handled the request
                                        if !no_wait  {
                                            self.channel.shared.outgoing_tx
                                            .send((self.channel.channel_id(), CancelOk::new(consumer_tag.try_into().unwrap()).into_frame()))
                                            .await.unwrap();
                                        }
                                      }
                                    };
                                } else {
                                    error!("callback not registered on channel {}", self.channel);
                                }
                            }
                            // in confirmed mode
                            Frame::Ack(_, ack) => {
                                if let Some(ref mut cb) = self.callback {
                                    cb.publish_ack(&self.channel, ack).await;
                                } else {
                                    error!("callback not registered on channel {}", self.channel);
                                }
                            }
                            Frame::Nack(_, nack) => {
                                if let Some(ref mut cb) = self.callback {
                                    cb.publish_nack(&self.channel, nack).await;
                                } else {
                                    error!("callback not registered on channel {}", self.channel);
                                }                            }
                            _ => unreachable!("dispatcher of channel {} receive unexpected frame {}", self.channel, frame),
                        }
                    }
                    else => {
                        break;
                    }

                }
            }
            info!("exit dispatcher of channel {}", self.channel);
        });
    }
}
