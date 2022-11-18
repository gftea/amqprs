use crate::{
    api::error::Error,
    frame::{Frame, TxSelect, TxSelectOk, TxCommit, TxCommitOk, TxRollback, TxRollbackOk},
};

use super::{Channel, Result};

impl Channel {
    pub async fn tx_select(&self) -> Result<()> {
        let select = TxSelect;

        let responder_rx = self.register_responder(TxSelectOk::header()).await?;

        let _method = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, select.into_frame()),
            responder_rx,
            Frame::SelectOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }
    
    
    pub async fn tx_commit(&self) -> Result<()> {
        let select = TxCommit;

        let responder_rx = self.register_responder(TxCommitOk::header()).await?;

        let _method = synchronous_request!(
            self.shared.outgoing_tx,
            (self.shared.channel_id, select.into_frame()),
            responder_rx,
            Frame::SelectOk,
            Error::ChannelUseError
        )?;
        Ok(())
    }

    pub async fn tx_rollback(&self) -> Result<()> {
        let select = TxRollback;

        let responder_rx = self.register_responder(TxRollbackOk::header()).await?;

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
