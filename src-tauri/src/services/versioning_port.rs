use serde::{Deserialize, Serialize};

use crate::models::AppError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VcActor {
    Agent,
    User,
    System,
}

pub type PatchOp = serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcCommitInput {
    pub project_path: String,
    pub entity_id: String,
    pub expected_revision: i64,
    pub call_id: String,
    pub actor: VcActor,
    pub before_hash: String,
    pub after_json: serde_json::Value,
    pub patch_ops: Vec<PatchOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcCommitOutput {
    pub ok: bool,
    pub tx_id: String,
    pub revision_before: i64,
    pub revision_after: i64,
    pub after_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityHead {
    pub revision: i64,
    pub json_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_tx_id: Option<String>,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_snapshot_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackByRevisionInput {
    pub project_path: String,
    pub entity_id: String,
    pub target_revision: i64,
    pub call_id: String,
    pub actor: VcActor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackByCallIdInput {
    pub project_path: String,
    pub target_call_id: String,
    pub call_id: String,
    pub actor: VcActor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackOutput {
    pub ok: bool,
    pub tx_id: String,
    pub revision_before: i64,
    pub revision_after: i64,
    pub after_hash: String,
    pub rolled_back_to_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecoverOutput {
    pub ok: bool,
    pub repaired_tmp_files: i64,
    pub truncated_wal_bytes: i64,
    pub rebuilt_head_entities: i64,
    pub appended_call_index: i64,
    pub notes: Vec<String>,
}

pub trait VcCommitPort {
    fn commit_with_occ(&self, input: VcCommitInput) -> Result<VcCommitOutput, AppError>;

    fn get_current_head(&self, project_path: &str, entity_id: &str)
        -> Result<EntityHead, AppError>;

    fn rollback_by_revision(
        &self,
        input: RollbackByRevisionInput,
    ) -> Result<RollbackOutput, AppError>;

    fn rollback_by_call_id(&self, input: RollbackByCallIdInput)
        -> Result<RollbackOutput, AppError>;

    fn recover(&self, project_path: &str) -> Result<RecoverOutput, AppError>;
}
