use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TxSelect;
#[derive(Debug, Serialize, Deserialize)]
pub struct TxSelectOk;
#[derive(Debug, Serialize, Deserialize)]
pub struct TxCommit;
#[derive(Debug, Serialize, Deserialize)]
pub struct TxCommitOk;
#[derive(Debug, Serialize, Deserialize)]
pub struct TxRollback;
#[derive(Debug, Serialize, Deserialize)]
pub struct TxRollbackOk;
