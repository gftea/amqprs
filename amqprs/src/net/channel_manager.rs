use std::collections::BTreeMap;

use amqp_serde::types::{AmqpChannelId, ShortUint};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::frame::MethodHeader;

use super::{channel_id_repo::ChannelIdRepository, ChannelResource, IncomingMessage};

pub(super) struct ChannelManager {
    /// channel id allocator and manager
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
                if self.channel_id_repo.reserve(&id) {
                    match self.resource.insert(id, resource) {
                        Some(_old) => unreachable!("Implementation error"),
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
                    Some(_old) => unreachable!("Implementation error"),
                    None => id,
                }
            }
        };

        Some(id)
    }

    /// remove channel resource, when channel to be closed
    pub fn remove_resource(&mut self, channel_id: &AmqpChannelId) -> Option<ChannelResource> {
        assert_eq!(
            true,
            self.channel_id_repo.release(channel_id),
            "Implementation error"
        );
        // remove responder means channel is to be  closed
        self.resource.remove(channel_id)
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

    pub fn get_dispatcher(&self, channel_id: &AmqpChannelId) -> Option<&Sender<IncomingMessage>> {
        self.resource.get(channel_id)?.dispatcher.as_ref()
    }
}
