use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Actor {
    Agent,
    User,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub ok: bool,
    pub revision_before: i64,
    pub revision_after: i64,
    pub json_hash_after: String,
    pub tx_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOp {
    InsertBlocks {
        #[serde(skip_serializing_if = "Option::is_none")]
        after_block_id: Option<String>,
        blocks: Vec<serde_json::Value>,
    },
    UpdateBlock {
        block_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        before: Option<serde_json::Value>,
        after: serde_json::Value,
    },
    DeleteBlocks {
        block_ids: Vec<String>,
    },
    MoveBlock {
        block_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        after_block_id: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}
