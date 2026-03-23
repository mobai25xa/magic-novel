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
}

impl Default for ToolSchemaContext {
    fn default() -> Self {
        Self {
            semantic_retrieval_enabled: false,
            clarification_mode: ClarificationMode::Interactive,
            available_skills: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCapability {
    WorkspaceRead,
    ContextRead,
    Search,
    KnowledgeRead,
    KnowledgeWrite,
    DraftWrite,
    StructureWrite,
    Review,
    Todo,
    AskUser,
    SkillActivation,
    Delegate,
    InspirationPatch,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolVisibility {
    pub interactive_only: bool,
    pub allow_in_delegate: bool,
    pub allow_in_workflow_job: bool,
    pub allow_in_review_gate: bool,
}

impl ToolVisibility {
    pub const fn everywhere() -> Self {
        Self {
            interactive_only: false,
            allow_in_delegate: true,
            allow_in_workflow_job: true,
            allow_in_review_gate: true,
        }
    }

    pub const fn user_interactive_only() -> Self {
        Self {
            interactive_only: true,
            allow_in_delegate: false,
            allow_in_workflow_job: false,
            allow_in_review_gate: false,
        }
    }

    pub const fn main_session_only() -> Self {
        Self {
            interactive_only: false,
            allow_in_delegate: false,
            allow_in_workflow_job: false,
            allow_in_review_gate: false,
        }
    }

    pub const fn no_delegate() -> Self {
        Self {
            interactive_only: false,
            allow_in_delegate: false,
            allow_in_workflow_job: true,
            allow_in_review_gate: true,
        }
    }
}

impl Default for ToolVisibility {
    fn default() -> Self {
        Self::everywhere()
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
    pub capabilities: &'static [ToolCapability],
    pub visibility: ToolVisibility,
}

/// Trait that each tool implements, co-locating manifest, schema, and execution.
pub trait ToolDefinition: Send + Sync {
    /// Tool name as seen by the LLM (e.g. "context_read", "draft_write")
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
