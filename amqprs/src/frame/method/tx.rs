use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectTx;
#[derive(Debug, Serialize, Deserialize)]
pub struct SelectTxOk;
#[derive(Debug, Serialize, Deserialize)]
pub struct Commit;
#[derive(Debug, Serialize, Deserialize)]
pub struct CommitOk;
#[derive(Debug, Serialize, Deserialize)]
pub struct Rollback;
#[derive(Debug, Serialize, Deserialize)]
pub struct RollbackOk;
