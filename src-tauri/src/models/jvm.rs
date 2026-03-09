use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub project_path: String,
    pub chapter_path: String,
    #[serde(default)]
    pub include_block_hints: Option<bool>,
    pub call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub chapter_id: String,
    pub revision: i64,
    pub json_hash: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSnapshotV2 {
    pub snapshot_id: String,
    pub block_count: u32,
    pub blocks: Vec<SnapshotBlockV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotBlockV2 {
    pub block_id: String,
    pub block_type: String,
    pub order: u32,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotBlockInputV2 {
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOpV2 {
    ReplaceBlock {
        block_id: String,
        markdown: String,
    },
    DeleteBlock {
        block_id: String,
    },
    InsertBefore {
        block_id: String,
        blocks: Vec<SnapshotBlockInputV2>,
    },
    InsertAfter {
        block_id: String,
        blocks: Vec<SnapshotBlockInputV2>,
    },
    AppendBlocks {
        blocks: Vec<SnapshotBlockInputV2>,
    },
    ReplaceRange {
        start_block_id: String,
        end_block_id: String,
        blocks: Vec<SnapshotBlockInputV2>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreviewMode {
    Replace,
    PatchPreferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRequest {
    pub project_path: String,
    pub chapter_path: String,
    pub base_revision: i64,
    pub call_id: String,
    pub mode: PreviewMode,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResult {
    pub ok: bool,
    pub patch_ops: Vec<PatchOp>,
    pub diagnostics: Vec<Diagnostic>,
    pub diff_summary: Vec<String>,
    pub revision_before: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Actor {
    Agent,
    User,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    pub project_path: String,
    pub chapter_path: String,
    pub base_revision: i64,
    pub call_id: String,
    pub patch_ops: Vec<PatchOp>,
    pub actor: Actor,
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
