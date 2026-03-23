//! Tool Contracts - Unified I/O types for agent tools
//!
//! This module defines the shared transport contract that both TypeScript and
//! Rust must agree upon for tool invocation and results.

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

// ── Shared search types ─────────────────────────────────────────────

/// Path scope allowlist used by search use-cases and `context_search`.
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
pub enum GrepMode {
    Keyword,
    Semantic,
    Hybrid,
}
