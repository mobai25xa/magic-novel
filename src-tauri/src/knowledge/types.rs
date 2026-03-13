use serde::{Deserialize, Serialize};

pub const KNOWLEDGE_SCHEMA_VERSION: i32 = 1;

// Conflict/error codes (stringly-typed to match spec/TS contracts)
pub const KNOWLEDGE_SOURCE_MISSING: &str = "KNOWLEDGE_SOURCE_MISSING";
pub const KNOWLEDGE_REVIEW_BLOCKED: &str = "KNOWLEDGE_REVIEW_BLOCKED";
pub const KNOWLEDGE_REVISION_CONFLICT: &str = "KNOWLEDGE_REVISION_CONFLICT";
pub const KNOWLEDGE_BRANCH_STALE: &str = "KNOWLEDGE_BRANCH_STALE";
pub const KNOWLEDGE_PROPOSAL_INVALID: &str = "KNOWLEDGE_PROPOSAL_INVALID";
pub const KNOWLEDGE_POLICY_CONFLICT: &str = "KNOWLEDGE_POLICY_CONFLICT";
pub const KNOWLEDGE_CANON_CONFLICT: &str = "KNOWLEDGE_CANON_CONFLICT";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeOp {
    Create,
    Update,
    Archive,
    Restore,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAcceptPolicy {
    AutoIfPass,
    Manual,
    OrchestratorOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeProposalBundle {
    pub schema_version: i32,
    pub bundle_id: String,
    pub scope_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    pub source_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_review_id: Option<String>,
    pub generated_at: i64,
    #[serde(default)]
    pub proposal_items: Vec<KnowledgeProposalItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeProposalItem {
    pub item_id: String,
    pub kind: String,
    pub op: KnowledgeOp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_revision: Option<i64>,
    #[serde(default)]
    pub fields: serde_json::Value,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<String>,
    pub change_reason: String,
    pub accept_policy: KnowledgeAcceptPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDeltaStatus {
    Proposed,
    Accepted,
    Applied,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeDecisionActor {
    User,
    Orchestrator,
}

impl Default for KnowledgeDecisionActor {
    fn default() -> Self {
        Self::User
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDeltaTarget {
    pub r#ref: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDeltaChange {
    pub item_id: String,
    pub op: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConflict {
    #[serde(rename = "type")]
    pub conflict_type: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRollbackKind {
    Soft,
    Hard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRollback {
    pub kind: KnowledgeRollbackKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDelta {
    pub schema_version: i32,
    pub knowledge_delta_id: String,
    pub status: KnowledgeDeltaStatus,
    pub scope_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    pub source_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_review_id: Option<String>,
    pub generated_at: i64,

    #[serde(default)]
    pub targets: Vec<KnowledgeDeltaTarget>,
    #[serde(default)]
    pub changes: Vec<KnowledgeDeltaChange>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<KnowledgeConflict>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_item_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_item_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback: Option<KnowledgeRollback>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionKnowledgeLatest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<KnowledgeProposalBundle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<KnowledgeDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDecisionInput {
    pub schema_version: i32,
    pub bundle_id: String,
    pub delta_id: String,
    #[serde(default)]
    pub actor: KnowledgeDecisionActor,
    #[serde(default)]
    pub accepted_item_ids: Vec<String>,
    #[serde(default)]
    pub rejected_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingKnowledgeDecision {
    pub schema_version: i32,
    pub bundle_id: String,
    pub delta_id: String,
    pub scope_ref: String,
    #[serde(default)]
    pub conflicts: Vec<KnowledgeConflict>,
    pub created_at: i64,
}
