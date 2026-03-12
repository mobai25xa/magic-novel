//! Agent Engine - Shared types

use serde::{Deserialize, Serialize};

pub const DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_PROVIDER: &str = "openai-compatible";

/// Stop reason for a turn
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Success,
    Cancel,
    Error,
    Limit,
    WaitingConfirmation,
    WaitingAskuser,
}

/// Token usage info from a LLM call
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub thinking_tokens: u64,
}

/// A tool call extracted from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// The ID assigned by the LLM (used for tool_result pairing)
    pub llm_call_id: String,
    /// Tool name (e.g. "read", "edit", "ls", "grep", "create")
    pub tool_name: String,
    /// Tool arguments as JSON
    pub args: serde_json::Value,
}

/// Autonomy level controlling legacy confirmation behavior mapping.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    /// All writes require confirmation
    Supervised,
    /// Only sensitive writes require confirmation
    SemiAutonomous,
    /// Sensitive writes can execute without confirmation
    Autonomous,
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        Self::SemiAutonomous
    }
}

/// Agent mode controlling which tools are visible to the LLM.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Full tool access — reading, writing, creating
    Writing,
    /// Read-only exploration — no edit/create, used for planning and review
    Planning,
}

impl Default for AgentMode {
    fn default() -> Self {
        Self::Writing
    }
}

/// User-facing approval policy for side-effecting tools.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    ConfirmWrites,
    Auto,
}

impl ApprovalMode {
    pub const fn to_autonomy_level(self) -> AutonomyLevel {
        match self {
            Self::ConfirmWrites => AutonomyLevel::SemiAutonomous,
            Self::Auto => AutonomyLevel::Autonomous,
        }
    }
}

impl Default for ApprovalMode {
    fn default() -> Self {
        Self::ConfirmWrites
    }
}

/// Clarification policy for interactive vs headless execution surfaces.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationMode {
    Interactive,
    HeadlessDefer,
}

impl ClarificationMode {
    pub const fn exposes_askuser(self) -> bool {
        matches!(self, Self::Interactive)
    }
}

impl Default for ClarificationMode {
    fn default() -> Self {
        Self::Interactive
    }
}

/// Input provided when resuming a suspended turn
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ResumeInput {
    #[serde(rename = "confirmation")]
    Confirmation { allowed: bool },
    #[serde(rename = "askuser")]
    AskUser { answers: serde_json::Value },
}

/// Result returned from agent_turn_start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurnStartResult {
    pub session_id: String,
    pub turn_id: u32,
    pub client_request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hydration_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_revision: Option<u64>,
}

/// Configuration for the agent loop
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoopConfig {
    /// Max rounds before forced stop (safety valve)
    pub max_rounds: u32,
    /// Max total tool calls across all rounds
    pub max_tool_calls: u32,
    /// Internal legacy confirmation mapping used by existing loop components
    pub autonomy_level: AutonomyLevel,
    /// Whether this loop can dispatch worker sub-loops (false for workers to prevent recursion)
    pub worker_dispatch_enabled: bool,
    /// Tool visibility mode for the current loop
    pub capability_mode: AgentMode,
    /// Approval policy for side-effecting tools
    pub approval_mode: ApprovalMode,
    /// Clarification policy for askuser exposure
    pub clarification_mode: ClarificationMode,
}

impl LoopConfig {
    pub fn headless_worker(max_rounds: u32, max_tool_calls: u32) -> Self {
        let approval_mode = ApprovalMode::Auto;
        Self {
            max_rounds,
            max_tool_calls,
            autonomy_level: approval_mode.to_autonomy_level(),
            worker_dispatch_enabled: false,
            capability_mode: AgentMode::Writing,
            approval_mode,
            clarification_mode: ClarificationMode::HeadlessDefer,
        }
    }
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_rounds: 25,
            max_tool_calls: 100,
            autonomy_level: AutonomyLevel::default(),
            worker_dispatch_enabled: false,
            capability_mode: AgentMode::Writing,
            approval_mode: ApprovalMode::ConfirmWrites,
            clarification_mode: ClarificationMode::Interactive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn approval_mode_maps_to_legacy_autonomy() {
        assert_eq!(
            ApprovalMode::ConfirmWrites.to_autonomy_level(),
            AutonomyLevel::SemiAutonomous
        );
        assert_eq!(
            ApprovalMode::Auto.to_autonomy_level(),
            AutonomyLevel::Autonomous
        );
    }

    #[test]
    fn loop_config_default_preserves_compatible_modes() {
        let config = LoopConfig::default();
        assert_eq!(config.capability_mode, AgentMode::Writing);
        assert_eq!(config.approval_mode, ApprovalMode::ConfirmWrites);
        assert_eq!(config.clarification_mode, ClarificationMode::Interactive);
        assert_eq!(config.autonomy_level, AutonomyLevel::SemiAutonomous);
    }

    #[test]
    fn headless_worker_defaults_to_auto_and_deferred_clarification() {
        let config = LoopConfig::headless_worker(10, 20);
        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.max_tool_calls, 20);
        assert_eq!(config.capability_mode, AgentMode::Writing);
        assert_eq!(config.approval_mode, ApprovalMode::Auto);
        assert_eq!(config.clarification_mode, ClarificationMode::HeadlessDefer);
        assert_eq!(config.autonomy_level, AutonomyLevel::Autonomous);
        assert!(!config.worker_dispatch_enabled);
    }

    #[test]
    fn resume_input_uses_askuser_tag_contract() {
        let parsed: ResumeInput = serde_json::from_value(json!({
            "kind": "askuser",
            "answers": { "ok": true }
        }))
        .expect("askuser payload should deserialize");

        match parsed {
            ResumeInput::AskUser { answers } => {
                assert_eq!(answers, json!({ "ok": true }));
            }
            _ => panic!("expected askuser variant"),
        }

        let serialized = serde_json::to_value(ResumeInput::Confirmation { allowed: true })
            .expect("confirmation should serialize");
        assert_eq!(serialized["kind"], "confirmation");
    }

    #[test]
    fn resume_input_rejects_legacy_ask_user_tag() {
        let parsed = serde_json::from_value::<ResumeInput>(json!({
            "kind": "ask_user",
            "answers": { "ok": true }
        }));

        assert!(parsed.is_err());
    }
}
