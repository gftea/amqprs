use std::collections::{BTreeMap, HashMap};

use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::frame::MethodHeader;

use super::{channel_id_repo::ChannelIdRepository, IncomingMessage};

pub(crate) struct ChannelResource {
    /// responder to acknowledge synchronous request
    /// responders are oneshot channel, which are not dedicated resource for channel
    pub responders: HashMap<&'static MethodHeader, oneshot::Sender<IncomingMessage>>,

    /// connection's default channel does not have dispatcher
    /// each channel has one and only one dispatcher
    pub dispatcher: Option<Sender<IncomingMessage>>,
}

impl ChannelResource {
    pub(crate) fn new(dispatcher: Option<Sender<IncomingMessage>>) -> Self {
        Self {
            responders: HashMap::new(),
            dispatcher,
            // amqp_channel,
            // callback: None,
        }
    }
}
pub(super) struct ChannelManager {
    /// channel id manager to allocate, reserve, and free id
    /// Keep the id manager out of AMQ connection type and use registeration machanism to manage id,
    /// which allows a slim Connection type that can be easily shared concurrently
    /// If we have the id manager in AMQ Connection type, then shared Connection object need to be mutable concurrently
    channel_id_repo: ChannelIdRepository,

    /// channel resource registery store
    resource: BTreeMap<AmqpChannelId, ChannelResource>,
}

impl ChannelManager {
    pub fn new(channel_max: ShortUint) -> Self {
        Self {
            channel_id_repo: ChannelIdRepository::new(channel_max),
            resource: BTreeMap::new(),
        }
    }
    /// Insert channel resource, when open a new channel
    pub fn insert_resource(
        &mut self,
        channel_id: Option<AmqpChannelId>,
        resource: ChannelResource,
    ) -> Option<AmqpChannelId> {
        let id = match channel_id {
            // reserve channel id as requested
            Some(id) => {
                // connection's default channel 0 is static reserved
                if id == 0 || self.channel_id_repo.reserve(id) {
                    match self.resource.insert(id, resource) {
                        Some(_old) => unreachable!("implementation error"),
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
                    Some(_old) => unreachable!("implementation error"),
                    None => id,
                }
            }
        };

        Some(id)
    }

    /// remove channel resource, when channel to be closed
    pub fn remove_resource(&mut self, channel_id: &AmqpChannelId) -> Option<ChannelResource> {
        assert!(
            self.channel_id_repo.release(*channel_id),
            "release a free id, implementation error"
        );
        // remove responder means channel is to be  closed
        self.resource.remove(channel_id)
    }

    pub fn get_dispatcher(&self, channel_id: &AmqpChannelId) -> Option<&Sender<IncomingMessage>> {
        self.resource.get(channel_id)?.dispatcher.as_ref()
    }

    pub fn insert_responder(
        &mut self,
        channel_id: &AmqpChannelId,
        method_header: &'static MethodHeader,
        responder: oneshot::Sender<IncomingMessage>,
    ) -> Option<oneshot::Sender<IncomingMessage>> {
        self.resource
            .get_mut(channel_id)?
            .responders
            .insert(method_header, responder)
    }

    pub fn remove_responder(
        &mut self,
        channel_id: &AmqpChannelId,
        method_header: &'static MethodHeader,
    ) -> Option<oneshot::Sender<IncomingMessage>> {
        self.resource
            .get_mut(channel_id)?
            .responders
            .remove(method_header)
    }
}
