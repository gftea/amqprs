//! API implementation of AMQP Channel
//!

use std::collections::{HashMap, VecDeque};

use tokio::{
    sync::{mpsc, oneshot},
    task::yield_now,
};

use crate::{
    api::{callbacks::ChannelCallback, channel::ReturnMessage},
    frame::{CancelOk, CloseChannelOk, FlowOk, Frame, MethodHeader},
    net::{ConnManagementCommand, IncomingMessage},
};
use tracing::{debug, error, trace};

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

/// Dispatcher for a AMQP channel.
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
            trace!(
                "dispatcher of channel {} starts up.",
                self.channel.channel_id()
            );
            // main loop of dispatcher
            loop {
                tokio::select! {
                    biased;

                    command = self.dispatcher_mgmt_rx.recv() => {
                        // handle command channel error
                        let cmd = match command {
                            None => break,
                            Some(v) => v,
                        };
                        // handle command
                        match cmd {
                            DispatcherManagementCommand::RegisterContentConsumer(cmd) => {
                                // TODO: check insert result
                                trace!("AsyncConsumer: {}, tx registered.", cmd.consumer_tag);
                                let consumer = self.get_or_new_consumer(&cmd.consumer_tag);
                                consumer.register_tx(cmd.consumer_tx);
                                // forward buffered messages
                                while !consumer.fifo.is_empty() {
                                    trace!("total buffered messages: {}.", consumer.fifo.len());
                                    let msg = consumer.pop().unwrap();
                                    if let Err(_err) = consumer.get_tx().unwrap().send(msg).await {
                                        error!("failed to forward message to consumer {}", &cmd.consumer_tag);
                                    }
                                }

                            },
                            DispatcherManagementCommand::UnregisterContentConsumer(cmd) => {
                                // TODO: check remove result
                                self.remove_consumer(&cmd.consumer_tag);

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
                            }
                        }
                    }
                    message = self.dispatcher_rx.recv() => {
                        // handle message channel error
                        let frame = match message {
                            None => break,
                            Some(v) => v,
                        };
                        // handle frames
                        match frame {
                            Frame::GetEmpty(_, get_empty) => {
                                self.state = State::GetEmpty;
                                if let Err(err) = self.get_content_responder.take().expect("get responder must be registered").send(get_empty.into_frame()).await {
                                    debug!("failed to dispatch GetEmpty frame, cause: {}.", err);
                                }
                            }
                            Frame::GetOk(_, get_ok) => {
                                self.state = State::GetOk;
                                if let Err(err) = self.get_content_responder.as_ref().expect("get responder must be registered").send(get_ok.into_frame()).await {
                                    debug!("failed to dispatch GetOk frame, cause: {}.", err);
                                }
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
                                        if let Err(err) = self.get_content_responder.as_ref().expect("get responder must be registered").send(header.into_frame()).await {
                                            debug!("failed to dispatch GetOk ContentHeader frame, cause: {}.", err);
                                        }
                                    },
                                    State::Return => return_buffer.basic_properties = Some(header.basic_properties),

                                    State::Initial | State::GetEmpty  => unreachable!("invalid dispatcher state"),
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
                                                    debug!("failed to dispatch message to consumer {}.", consumer_tag);
                                                }
                                            },
                                            None => {
                                                debug!("can't find consumer '{}', buffering message.", consumer_tag);
                                                consumer.push(consumer_message);
                                                // FIXME: try to yield for registering consumer
                                                //      not sure if it is necessary
                                                yield_now().await;
                                            },
                                        };
                                    }
                                    State::GetOk => {
                                        if let Err(err) = self.get_content_responder.take().expect("get responder must be registered").send(body.into_frame()).await {
                                            debug!("failed to dispatch GetOk ContentBody frame, cause: {}.", err);
                                        }
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
                                            debug!("channel {} callback not registered.", self.channel.channel_id());
                                        }
                                    },
                                    State::Initial | State::GetEmpty  => unreachable!("invalid dispatcher state"),
                                }


                            }
                            // Close channel response from server
                            Frame::CloseChannelOk(method_header, close_channel_ok) => {
                                self.responders.remove(method_header)
                                .expect("responder must be registered for CloseChannelOk")
                                .send(close_channel_ok.into_frame()).unwrap();

                                self.channel.set_is_open(false);
                                break;
                            }
                            // TODO:
                            | Frame::FlowOk(method_header, _)
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
                                            debug!(
                                                "failed to forward response frame {} to channel {}",
                                                response, self.channel.channel_id()
                                            );
                                        }
                                    }
                                    None => debug!(
                                        "No responder to forward frame {} to channel {}",
                                        frame, self.channel.channel_id()
                                    ),
                                }

                            }
                            //////////////////////////////////////////////////////////
                            // Method frames of asynchronous request

                            // Server request to close channel
                            Frame::CloseChannel(_, close_channel) => {
                                // callback
                                if let Some(ref mut cb) = self.callback {
                                    if let Err(err) = cb.close(&self.channel, close_channel).await {
                                      debug!("channel {} close callback error, cause: {}.", self.channel.channel_id(), err);
                                      // no response to server
                                      break;
                                    };
                                } else {
                                    debug!("channel {} callback not registered.", self.channel.channel_id());
                                }

                                // respond to server if no callback registered or callback succeed
                                self.channel.set_is_open(false);

                                self.channel.shared.outgoing_tx
                                .send((self.channel.channel_id(), CloseChannelOk::default().into_frame()))
                                .await.unwrap();

                                break;
                            }

                            Frame::Flow(_, flow) => {
                                // callback
                                if let Some(ref mut cb) = self.callback {
                                    match cb.flow(&self.channel, flow).await {
                                      Err(err) => {
                                        debug!("channel {} flow callback error, cause: {}.", self.channel.channel_id(), err);
                                        // no response to server
                                        break;
                                      }
                                      Ok(active) => {
                                         // respond to server that we have handled the request
                                         self.channel.shared.outgoing_tx
                                         .send((self.channel.channel_id(), FlowOk::new(active).into_frame()))
                                         .await.unwrap();
                                      }
                                    };
                                } else {
                                    debug!("channel {} callback not registered.", self.channel.channel_id());
                                }

                            }
                            Frame::Cancel(_, cancel) => {
                                // callback
                                if let Some(ref mut cb) = self.callback {
                                    let consumer_tag = cancel.consumer_tag().clone();
                                    let no_wait = cancel.no_wait();
                                    match cb.cancel(&self.channel, cancel).await {
                                      Err(err) => {
                                        debug!("channel {} cancel callback error, cause: {}.", self.channel.channel_id(), err);
                                        // no response to server
                                        break;
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
                                    debug!("channel {} callback not registered.", self.channel.channel_id());
                                }
                            }
                            // in confirmed mode
                            Frame::Ack(_, ack) => {

                                if let Some(ref mut cb) = self.callback {
                                    cb.publish_ack(&self.channel, ack).await;
                                } else {
                                    debug!("channel {} callback not registered.", self.channel.channel_id());
                                }
                            }
                            Frame::Nack(_, nack) => {
                                if let Some(ref mut cb) = self.callback {
                                    cb.publish_nack(&self.channel, nack).await;
                                } else {
                                    debug!("channel {} callback not registered.", self.channel.channel_id());
                                }                            }
                            _ => unreachable!("not acceptable frame for dispatcher: {:?}", frame),
                        }
                    }
                    else => {
                        break;
                    }

                }
            }
            let cmd = ConnManagementCommand::UnregisterChannelResource(self.channel.channel_id());
            debug!(
                "request to unregister channel resource {}.",
                self.channel.channel_id()
            );
            if let Err(err) = self.channel.shared.conn_mgmt_tx.send(cmd).await {
                error!("failed to unregister channel resource, cause: {}. Connection may be already closed!", err);
            }
            debug!("exit dispatcher of channel {}.", self.channel.channel_id());
        });
    }
}
