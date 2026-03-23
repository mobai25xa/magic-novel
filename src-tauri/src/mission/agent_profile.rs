use serde::{Deserialize, Serialize};

use crate::agent_engine::exposure_policy::CapabilityPolicy;
use crate::agent_engine::types::{AgentMode, ApprovalMode, ClarificationMode};

pub const DEFAULT_AGENT_MAX_ROUNDS: u32 = 20;
pub const DEFAULT_AGENT_MAX_TOOL_CALLS: u32 = 80;

pub use crate::agent_engine::exposure_policy::{CapabilityPreset, SessionSource};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AgentProfile {
    pub name: String,
    pub display_name: String,
    pub prompt_preset: String,
    pub mode: AgentMode,
    pub approval_mode: ApprovalMode,
    pub clarification_mode: ClarificationMode,
    pub capability_preset: CapabilityPreset,
    pub allow_delegate: bool,
    #[serde(default)]
    pub allow_skill_activation: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forced_tools: Vec<String>,
    pub max_rounds: u32,
    pub max_tool_calls: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl Default for AgentProfile {
    fn default() -> Self {
        Self {
            name: "general-worker".to_string(),
            display_name: "General Worker".to_string(),
            prompt_preset: String::new(),
            mode: AgentMode::Writing,
            approval_mode: ApprovalMode::Auto,
            clarification_mode: ClarificationMode::HeadlessDefer,
            capability_preset: CapabilityPreset::HeadlessWriter,
            allow_delegate: false,
            allow_skill_activation: false,
            hidden_tools: Vec::new(),
            forced_tools: Vec::new(),
            max_rounds: DEFAULT_AGENT_MAX_ROUNDS,
            max_tool_calls: DEFAULT_AGENT_MAX_TOOL_CALLS,
            model: None,
        }
    }
}

impl AgentProfile {
    pub fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self.display_name = self.display_name.trim().to_string();
        self.prompt_preset = self.prompt_preset.trim().to_string();
        self.max_rounds = self.max_rounds.max(1);
        self.max_tool_calls = self.max_tool_calls.max(1);
        self.hidden_tools = normalize_tool_names(self.hidden_tools.clone());
        self.forced_tools = normalize_tool_names(self.forced_tools.clone());
        self.model = self
            .model
            .take()
            .and_then(|value| normalize_optional_string(Some(value)));
        self
    }

    pub fn system_prompt(&self) -> &str {
        self.prompt_preset.trim()
    }

    pub fn capability_policy(&self) -> CapabilityPolicy {
        let mut policy = CapabilityPolicy::new(self.capability_preset);
        policy.allow_delegate = self.allow_delegate;
        policy.allow_skill_activation = self.allow_skill_activation;
        policy.hidden_tools = self.hidden_tools.clone();
        policy.forced_tools = self.forced_tools.clone();
        policy.normalized()
    }

    pub fn infer_capability_preset(legacy_tools: &[String]) -> CapabilityPreset {
        let normalized = normalize_tool_names(legacy_tools.to_vec());
        if normalized.is_empty() {
            return CapabilityPreset::HeadlessWriter;
        }

        if normalized.iter().any(|tool| {
            matches!(
                tool.as_str(),
                "draft_write" | "structure_edit" | "knowledge_write"
            )
        }) {
            return CapabilityPreset::HeadlessWriter;
        }

        let summary_only = normalized.iter().all(|tool| {
            matches!(
                tool.as_str(),
                "workspace_map" | "context_read" | "context_search" | "todowrite"
            )
        });
        if summary_only {
            return CapabilityPreset::SummaryOnly;
        }

        if normalized
            .iter()
            .any(|tool| matches!(tool.as_str(), "review_check" | "knowledge_read"))
        {
            return CapabilityPreset::ReadOnlyReviewer;
        }

        CapabilityPreset::HeadlessWriter
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

pub fn normalize_tool_names(raw: Vec<String>) -> Vec<String> {
    let mut out = raw
        .into_iter()
        .map(|tool| tool.trim().to_ascii_lowercase())
        .filter(|tool| !tool.is_empty())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_profile_normalizes_strings_and_limits() {
        let profile = AgentProfile {
            name: " general ".to_string(),
            display_name: " General ".to_string(),
            prompt_preset: " prompt ".to_string(),
            max_rounds: 0,
            max_tool_calls: 0,
            model: Some(" gpt-test ".to_string()),
            hidden_tools: vec![" askuser ".to_string(), "ASKUSER".to_string()],
            forced_tools: vec![" skill ".to_string(), "SKILL".to_string()],
            ..AgentProfile::default()
        }
        .normalized();

        assert_eq!(profile.name, "general");
        assert_eq!(profile.display_name, "General");
        assert_eq!(profile.prompt_preset, "prompt");
        assert_eq!(profile.max_rounds, 1);
        assert_eq!(profile.max_tool_calls, 1);
        assert_eq!(profile.model.as_deref(), Some("gpt-test"));
        assert_eq!(profile.hidden_tools, vec!["askuser".to_string()]);
        assert_eq!(profile.forced_tools, vec!["skill".to_string()]);
    }

    #[test]
    fn infer_capability_preset_prefers_write_tools() {
        let preset = AgentProfile::infer_capability_preset(&[
            "context_read".to_string(),
            "draft_write".to_string(),
        ]);
        assert_eq!(preset, CapabilityPreset::HeadlessWriter);
    }

    #[test]
    fn infer_capability_preset_detects_summary_only() {
        let preset = AgentProfile::infer_capability_preset(&[
            "context_read".to_string(),
            "todowrite".to_string(),
        ]);
        assert_eq!(preset, CapabilityPreset::SummaryOnly);
    }

    #[test]
    fn capability_policy_carries_hidden_and_forced_overrides() {
        let profile = AgentProfile {
            capability_preset: CapabilityPreset::ReadOnlyReviewer,
            hidden_tools: vec!["review_check".to_string()],
            forced_tools: vec!["todowrite".to_string()],
            ..AgentProfile::default()
        };

        let policy = profile.capability_policy();
        assert_eq!(policy.hidden_tools, vec!["review_check".to_string()]);
        assert_eq!(policy.forced_tools, vec!["todowrite".to_string()]);
    }

    #[test]
    fn capability_policy_preserves_skill_activation_flag() {
        let profile = AgentProfile {
            capability_preset: CapabilityPreset::HeadlessWriter,
            allow_skill_activation: true,
            ..AgentProfile::default()
        }
        .normalized();

        let policy = profile.capability_policy();

        assert_eq!(policy.preset, CapabilityPreset::HeadlessWriter);
        assert!(policy.allow_skill_activation);
        assert!(!policy.allow_delegate);
    }
}
