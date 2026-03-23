//! Shared contract seed for role templates.
//!
//! `RoleProfile` is a reusable role template, not a concrete delegate run.
//! Wave 1 keeps legacy `AgentProfile` call sites intact and provides
//! conversions so later phases can migrate to the frozen naming without
//! re-breaking existing mission code.

use serde::{Deserialize, Serialize};

use crate::agent_engine::types::{AgentMode, ApprovalMode, ClarificationMode};

pub use crate::agent_engine::exposure_policy::{CapabilityPreset, SessionSource};

use super::agent_profile::{
    normalize_tool_names, AgentProfile, DEFAULT_AGENT_MAX_ROUNDS, DEFAULT_AGENT_MAX_TOOL_CALLS,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RoleProfile {
    pub profile_id: String,
    pub display_name: String,
    pub system_prompt: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub max_rounds: u32,
    pub max_tool_calls: u32,
}

impl Default for RoleProfile {
    fn default() -> Self {
        Self {
            profile_id: "general-worker".to_string(),
            display_name: "General Worker".to_string(),
            system_prompt: String::new(),
            mode: AgentMode::Writing,
            approval_mode: ApprovalMode::Auto,
            clarification_mode: ClarificationMode::HeadlessDefer,
            capability_preset: CapabilityPreset::HeadlessWriter,
            allow_delegate: false,
            allow_skill_activation: false,
            hidden_tools: Vec::new(),
            forced_tools: Vec::new(),
            model: None,
            max_rounds: DEFAULT_AGENT_MAX_ROUNDS,
            max_tool_calls: DEFAULT_AGENT_MAX_TOOL_CALLS,
        }
    }
}

impl RoleProfile {
    pub fn normalized(mut self) -> Self {
        self.profile_id = self.profile_id.trim().to_string();
        self.display_name = self.display_name.trim().to_string();
        self.system_prompt = self.system_prompt.trim().to_string();
        self.hidden_tools = normalize_tool_names(self.hidden_tools);
        self.forced_tools = normalize_tool_names(self.forced_tools);
        self.max_rounds = self.max_rounds.max(1);
        self.max_tool_calls = self.max_tool_calls.max(1);
        self.model = normalize_optional_string(self.model);
        self
    }

    pub fn to_agent_profile(&self) -> AgentProfile {
        AgentProfile {
            name: self.profile_id.clone(),
            display_name: self.display_name.clone(),
            prompt_preset: self.system_prompt.clone(),
            mode: self.mode,
            approval_mode: self.approval_mode,
            clarification_mode: self.clarification_mode,
            capability_preset: self.capability_preset,
            allow_delegate: self.allow_delegate,
            allow_skill_activation: self.allow_skill_activation,
            hidden_tools: self.hidden_tools.clone(),
            forced_tools: self.forced_tools.clone(),
            max_rounds: self.max_rounds,
            max_tool_calls: self.max_tool_calls,
            model: self.model.clone(),
        }
        .normalized()
    }
}

impl From<&AgentProfile> for RoleProfile {
    fn from(value: &AgentProfile) -> Self {
        Self {
            profile_id: value.name.clone(),
            display_name: value.display_name.clone(),
            system_prompt: value.prompt_preset.clone(),
            mode: value.mode,
            approval_mode: value.approval_mode,
            clarification_mode: value.clarification_mode,
            capability_preset: value.capability_preset,
            allow_delegate: value.allow_delegate,
            allow_skill_activation: value.allow_skill_activation,
            hidden_tools: value.hidden_tools.clone(),
            forced_tools: value.forced_tools.clone(),
            model: value.model.clone(),
            max_rounds: value.max_rounds,
            max_tool_calls: value.max_tool_calls,
        }
        .normalized()
    }
}

impl From<RoleProfile> for AgentProfile {
    fn from(value: RoleProfile) -> Self {
        value.to_agent_profile()
    }
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_profile_normalizes_strings_and_limits() {
        let profile = RoleProfile {
            profile_id: " writer ".to_string(),
            display_name: " Writer ".to_string(),
            system_prompt: " prompt ".to_string(),
            hidden_tools: vec![" AskUser ".to_string(), "askuser".to_string()],
            forced_tools: vec!["Skill".to_string(), "skill ".to_string()],
            model: Some(" gpt-test ".to_string()),
            max_rounds: 0,
            max_tool_calls: 0,
            ..RoleProfile::default()
        }
        .normalized();

        assert_eq!(profile.profile_id, "writer");
        assert_eq!(profile.display_name, "Writer");
        assert_eq!(profile.system_prompt, "prompt");
        assert_eq!(profile.hidden_tools, vec!["askuser".to_string()]);
        assert_eq!(profile.forced_tools, vec!["skill".to_string()]);
        assert_eq!(profile.model.as_deref(), Some("gpt-test"));
        assert_eq!(profile.max_rounds, 1);
        assert_eq!(profile.max_tool_calls, 1);
    }

    #[test]
    fn role_profile_roundtrips_with_agent_profile() {
        let agent_profile = AgentProfile {
            name: "delegate".to_string(),
            display_name: "Delegate".to_string(),
            prompt_preset: "system".to_string(),
            allow_delegate: true,
            allow_skill_activation: true,
            hidden_tools: vec!["review_check".to_string()],
            forced_tools: vec!["todowrite".to_string()],
            max_rounds: 7,
            max_tool_calls: 21,
            model: Some("gpt-test".to_string()),
            ..AgentProfile::default()
        }
        .normalized();

        let role_profile = RoleProfile::from(&agent_profile);
        let restored = role_profile.to_agent_profile();

        assert_eq!(role_profile.profile_id, "delegate");
        assert_eq!(role_profile.system_prompt, "system");
        assert!(role_profile.allow_skill_activation);
        assert_eq!(restored.name, agent_profile.name);
        assert_eq!(restored.prompt_preset, agent_profile.prompt_preset);
        assert_eq!(restored.hidden_tools, agent_profile.hidden_tools);
    }
}
