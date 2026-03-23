//! Layer B: Mode — interactive / exec / spec constraints.
//!
//! Maps plan_03 modes to existing engineering switches:
//!   - Spec       → capability_mode=Planning  (read-only, no write tools)
//!   - Interactive → clarification_mode=Interactive (askuser available)
//!   - Exec       → clarification_mode=HeadlessDefer (no askuser, worker default)

use crate::agent_engine::types::{AgentMode, ClarificationMode};

/// Prompt mode aligned with plan_03 terminology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    /// User-facing interactive session. askuser is available.
    Interactive,
    /// Headless execution (worker loops). No askuser, autonomous.
    Exec,
    /// Read-only analysis / planning. No write tools exposed.
    Spec,
}

impl PromptMode {
    /// Derive PromptMode from existing engineering switches.
    pub fn from_engine_modes(capability: AgentMode, clarification: ClarificationMode) -> Self {
        match capability {
            AgentMode::Planning => Self::Spec,
            AgentMode::Writing => match clarification {
                ClarificationMode::Interactive => Self::Interactive,
                ClarificationMode::HeadlessDefer => Self::Exec,
            },
        }
    }

    /// Label used in reminder fields.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Exec => "exec",
            Self::Spec => "spec",
        }
    }
}

/// Render mode-specific prompt text.
pub fn render_mode(mode: &PromptMode) -> String {
    match mode {
        PromptMode::Interactive => INTERACTIVE_MODE.to_string(),
        PromptMode::Exec => EXEC_MODE.to_string(),
        PromptMode::Spec => SPEC_MODE.to_string(),
    }
}

const INTERACTIVE_MODE: &str = r#"## Mode: interactive
- You are in interactive mode. The user is present and can answer questions.
- Use askuser when intent, target, or constraints are ambiguous before writing.
- Confirm risky or large-scale changes with the user before executing."#;

const EXEC_MODE: &str = r#"## Mode: exec
- You are in autonomous execution mode. No user is available to answer questions.
- Do NOT call askuser. Make reasonable decisions based on available context.
- If critical information is missing, record the gap via todowrite and proceed with best-effort."#;

const SPEC_MODE: &str = r#"## Mode: spec
- You are in read-only analysis mode. You MUST NOT modify any content.
- Do NOT use structure_edit, draft_write, or knowledge_write.
- Only use read/search tools: workspace_map, context_read, context_search, knowledge_read, review_check.
- Your role is to analyze, plan, and report. Output analysis and recommendations as text."#;
