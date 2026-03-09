use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::services::versioning_port::{EntityHead, VcActor};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeadState {
    pub project_path: String,
    pub last_event_seq: i64,
    pub entities: HashMap<String, EntityHead>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WalStatus {
    Ok,
    Failed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WalType {
    Preview,
    Begin,
    Commit,
    Rollback,
    Snapshot,
    RecoveryRepair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalRecord {
    pub event_seq: i64,
    pub ts: i64,
    pub r#type: WalType,
    pub tx_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<VcActor>,
    pub entity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_hash: Option<String>,
    pub status: WalStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallIndexRecord {
    pub call_id: String,
    pub entity_id: String,
    pub tx_id: String,
    pub from_revision: i64,
    pub to_revision: i64,
    pub event_seq: i64,
    pub ts: i64,
}
