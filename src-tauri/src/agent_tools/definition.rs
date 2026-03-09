//! Unified Tool Definition trait
//!
//! Each tool co-locates its manifest, schema, and execution logic.

use serde_json::Value;

use crate::agent_engine::types::ClarificationMode;

use super::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};

/// Default tool timeout in milliseconds (30 seconds).
pub const DEFAULT_TOOL_TIMEOUT_MS: u64 = 30_000;

/// Dynamic context used to build tool schemas.
#[derive(Debug, Clone)]
pub struct ToolSchemaContext {
    pub semantic_retrieval_enabled: bool,
    pub clarification_mode: ClarificationMode,
    pub available_skills: Vec<String>,
    pub available_workers: Vec<String>,
}

impl Default for ToolSchemaContext {
    fn default() -> Self {
        Self {
            semantic_retrieval_enabled: false,
            clarification_mode: ClarificationMode::Interactive,
            available_skills: Vec::new(),
            available_workers: Vec::new(),
        }
    }
}

/// Manifest metadata for a tool.
#[derive(Debug, Clone)]
pub struct ToolManifest {
    pub id: &'static str,
    pub llm_name: &'static str,
    pub domain: ToolDomain,
    pub risk_level: RiskLevel,
    pub confirmation: ConfirmationPolicy,
    pub idempotency: IdempotencyPolicy,
    pub parallel_safe: bool,
    /// Maximum execution time in milliseconds before the scheduler aborts the call.
    pub timeout_ms: u64,
}

/// Trait that each tool implements, co-locating manifest, schema, and execution.
pub trait ToolDefinition: Send + Sync {
    /// Tool name as seen by the LLM (e.g. "read", "edit")
    fn name(&self) -> &'static str;

    /// Manifest metadata (domain, risk, confirmation, parallel_safe)
    fn manifest(&self) -> ToolManifest;

    /// LLM-facing description text. Embedded in the tool schema sent to the model.
    /// Should include DO/DO NOT boundaries, PERFORMANCE TIPs, cross-references, etc.
    fn description(&self) -> &'static str {
        ""
    }

    /// OpenAI-compatible JSON schema for the LLM.
    /// Returns None when the tool should be hidden for the current context.
    fn schema(&self, _context: &ToolSchemaContext) -> Option<Value> {
        None
    }

    /// Whether this tool's execution is handled externally (e.g. askuser by scheduler, skill by loop_engine).
    fn externally_handled(&self) -> bool {
        false
    }
}
