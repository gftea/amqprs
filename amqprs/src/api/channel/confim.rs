use crate::{
    api::error::Error,
    frame::{Frame, Select, SelectOk},
};

use super::{Channel, Result};

#[derive(Debug, Clone)]
pub struct ConfirmSelectArguments {
    pub no_wait: bool,
}

impl ConfirmSelectArguments {
    pub fn new() -> Self {
        Self { no_wait: false }
    }
}

impl Channel {
    pub async fn confirm_select(&self, args: ConfirmSelectArguments) -> Result<()> {
        let select = Select::new(args.no_wait);
        if args.no_wait {
            self.shared
                .outgoing_tx
                .send((self.shared.channel_id, select.into_frame()))
                .await?;
            Ok(())
        } else {
            let responder_rx = self.register_responder(SelectOk::header()).await?;

            let _method = synchronous_request!(
                self.shared.outgoing_tx,
                (self.shared.channel_id, select.into_frame()),
                responder_rx,
                Frame::SelectOk,
                Error::ChannelUseError
            )?;
            Ok(())
        }
    }
}
