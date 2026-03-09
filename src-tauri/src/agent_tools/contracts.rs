//! Tool Contracts - Unified I/O types for agent tools
//!
//! Based on tool_contract.md v2
//!
//! This module defines the contract types that both TypeScript and Rust
//! must agree upon for tool invocation and results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDomain {
    Novel,
    System,
    Utility,
    Mode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationPolicy {
    Never,
    SensitiveWrite,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyPolicy {
    Required,
    Optional,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvokeRequest<T = serde_json::Value> {
    pub tool: String,
    pub call_id: String,
    pub turn_id: u32,
    pub actor: Actor,
    pub input: T,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ToolContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Agent,
    User,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult<T = serde_json::Value> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ToolError>,
    pub meta: ToolMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub fault_domain: FaultDomain,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultDomain {
    Tool,
    Validation,
    Policy,
    Jvm,
    Vc,
    Io,
    Network,
    Auth,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    pub tool: String,
    pub call_id: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_before: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_after: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_set: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_set: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInput {
    pub project_path: String,
    #[serde(default)]
    pub kind: Option<CreateKind>,
    #[serde(default)]
    pub volume_path: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub cwd: String,
    #[serde(default = "default_node_kind")]
    pub node_kind: NodeKind,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_content_format")]
    pub content_format: ContentFormat,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOutput {
    pub created_kind: CreateKind,
    pub path: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_after: Option<u64>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadInput {
    pub project_path: String,
    pub path: String,
    #[serde(default)]
    pub kind: Option<ReadKind>,
    #[serde(default = "default_read_view")]
    pub view: ViewFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadOutput {
    pub path: String,
    pub kind: NodeKind,
    pub revision: u64,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<ChapterSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_json: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSnapshot {
    pub snapshot_id: String,
    pub block_count: u32,
    pub blocks: Vec<SnapshotBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotBlock {
    pub block_id: String,
    pub block_type: String,
    pub order: u32,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditInput {
    pub project_path: String,
    pub path: String,
    #[serde(default)]
    pub target: Option<EditTarget>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub target_words: Option<i32>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub pinned_assets: Option<Vec<crate::models::ChapterAssetRef>>,
    #[serde(default)]
    pub base_revision: u64,
    #[serde(default)]
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub ops: Vec<EditOp>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_actor")]
    pub actor: Actor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOutput {
    pub mode: EditResultMode,
    pub accepted: bool,
    pub path: String,
    pub revision_before: u64,
    pub revision_after: u64,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub diff_summary: Vec<DiffSummary>,
    #[serde(default)]
    pub changed_block_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<String>,
    pub hash_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotBlockInput {
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum EditOp {
    ReplaceBlock {
        block_id: String,
        markdown: String,
    },
    DeleteBlock {
        block_id: String,
    },
    InsertBefore {
        block_id: String,
        blocks: Vec<SnapshotBlockInput>,
    },
    InsertAfter {
        block_id: String,
        blocks: Vec<SnapshotBlockInput>,
    },
    AppendBlocks {
        blocks: Vec<SnapshotBlockInput>,
    },
    ReplaceRange {
        start_block_id: String,
        end_block_id: String,
        blocks: Vec<SnapshotBlockInput>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteInput {
    pub project_path: String,
    pub kind: DeleteKind,
    pub path: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveInput {
    pub project_path: String,
    pub chapter_path: String,
    pub target_volume_path: String,
    pub target_index: i32,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsInput {
    pub project_path: String,
    pub cwd: String,
    #[serde(default = "default_depth")]
    pub depth: u32,
    #[serde(default)]
    pub include_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsOutput {
    pub cwd: String,
    pub items: Vec<LsItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsItem {
    pub kind: NodeKind,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub child_count: u32,
    pub revision: u64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepInput {
    pub project_path: String,
    pub query: String,
    #[serde(default = "default_grep_mode")]
    pub mode: GrepMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<GrepScope>,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepScope {
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepOutput {
    pub hits: Vec<GrepHit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_notice: Option<GrepSemanticNotice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepSemanticNotice {
    pub semantic_retrieval_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepHit {
    pub path: String,
    pub score: f64,
    pub snippet: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Folder,
    File,
    DomainObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateKind {
    Volume,
    Chapter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadKind {
    Volume,
    Chapter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditTarget {
    VolumeMeta,
    ChapterMeta,
    ChapterContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteKind {
    Volume,
    Chapter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentFormat {
    Text,
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewFormat {
    Meta,
    Snapshot,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditResultMode {
    Preview,
    Commit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrepMode {
    Keyword,
    Semantic,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub operation: String,
    pub description: String,
}

fn default_content_format() -> ContentFormat {
    ContentFormat::Text
}

fn default_node_kind() -> NodeKind {
    NodeKind::File
}

fn default_read_view() -> ViewFormat {
    ViewFormat::Snapshot
}

fn default_actor() -> Actor {
    Actor::Agent
}

fn default_depth() -> u32 {
    1
}

fn default_grep_mode() -> GrepMode {
    GrepMode::Keyword
}

fn default_top_k() -> u32 {
    10
}
