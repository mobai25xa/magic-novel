use serde::{Deserialize, Serialize};

use crate::services::global_config::WorkerDefinition;

use super::agent_profile::{normalize_tool_names, AgentProfile, CapabilityPreset};

pub const DEFAULT_WORKER_MAX_ROUNDS: u32 = 20;
pub const DEFAULT_WORKER_MAX_TOOL_CALLS: u32 = 80;

pub fn agent_profile_from_definition(def: &WorkerDefinition) -> AgentProfile {
    AgentProfile {
        name: def.name.trim().to_string(),
        display_name: def.display_name.trim().to_string(),
        prompt_preset: def.system_prompt.trim().to_string(),
        mode: def.mode,
        approval_mode: def.approval_mode,
        clarification_mode: def.clarification_mode,
        capability_preset: def.capability_preset,
        allow_delegate: def.allow_delegate,
        allow_skill_activation: def.allow_skill_activation,
        hidden_tools: normalize_tool_names(def.hidden_tools.clone()),
        forced_tools: normalize_tool_names(def.forced_tools.clone()),
        max_rounds: def.max_rounds.unwrap_or(DEFAULT_WORKER_MAX_ROUNDS).max(1),
        max_tool_calls: def
            .max_tool_calls
            .unwrap_or(DEFAULT_WORKER_MAX_TOOL_CALLS)
            .max(1),
        model: def
            .model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
    }
    .normalized()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfileSummary {
    pub name: String,
    pub display_name: String,
    pub mode: crate::agent_engine::types::AgentMode,
    pub approval_mode: crate::agent_engine::types::ApprovalMode,
    pub clarification_mode: crate::agent_engine::types::ClarificationMode,
    pub capability_preset: CapabilityPreset,
    pub allow_delegate: bool,
    pub allow_skill_activation: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forced_tools: Vec<String>,
    pub max_rounds: u32,
    pub max_tool_calls: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub system_prompt_hash: String,
    pub profile_hash: String,
}

impl AgentProfileSummary {
    pub fn from_agent_profile(profile: &AgentProfile) -> Self {
        let system_prompt_hash = hash_fnv64(profile.system_prompt());
        let profile_hash = serde_json::to_string(profile)
            .map(|json| hash_fnv64(&json))
            .unwrap_or_else(|_| system_prompt_hash.clone());

        Self {
            name: profile.name.clone(),
            display_name: profile.display_name.clone(),
            mode: profile.mode,
            approval_mode: profile.approval_mode,
            clarification_mode: profile.clarification_mode,
            capability_preset: profile.capability_preset,
            allow_delegate: profile.allow_delegate,
            allow_skill_activation: profile.allow_skill_activation,
            hidden_tools: profile.hidden_tools.clone(),
            forced_tools: profile.forced_tools.clone(),
            max_rounds: profile.max_rounds,
            max_tool_calls: profile.max_tool_calls,
            model: profile.model.clone(),
            system_prompt_hash,
            profile_hash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRunEntry {
    pub schema_version: i32,
    pub ts: i64,
    pub mission_id: String,
    pub feature_id: String,
    pub worker_id: String,
    pub attempt: u32,
    pub profile: AgentProfileSummary,
    pub provider: String,
    pub model: String,
}

pub fn builtin_general_worker_profile() -> AgentProfile {
    AgentProfile {
        name: "general-worker".to_string(),
        display_name: "General Worker".to_string(),
        prompt_preset: "You are a mission worker. Complete the assigned feature safely and efficiently.\n\
If you must make assumptions, state them explicitly in your final summary.\n\
Prefer small, verifiable steps. If blocked, produce a concise failure summary with actionable next steps."
            .to_string(),
        capability_preset: CapabilityPreset::HeadlessWriter,
        allow_skill_activation: true,
        ..AgentProfile::default()
    }
    .normalized()
}

pub fn builtin_integrator_worker_profile() -> AgentProfile {
    AgentProfile {
        name: "integrator".to_string(),
        display_name: "Integrator".to_string(),
        prompt_preset: "You are an integrator worker. Your job is to read mission artifacts and produce a final mission summary.\n\
Summarize what completed successfully, what failed, and what remains actionable.\n\
Do not modify project files unless explicitly required."
            .to_string(),
        mode: crate::agent_engine::types::AgentMode::Planning,
        capability_preset: CapabilityPreset::SummaryOnly,
        hidden_tools: vec!["knowledge_read".to_string(), "review_check".to_string()],
        forced_tools: vec!["todowrite".to_string()],
        max_rounds: 10,
        max_tool_calls: 30,
        ..AgentProfile::default()
    }
    .normalized()
}

fn hash_fnv64(text: &str) -> String {
    const OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;

    let mut hash = OFFSET_BASIS;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    format!("fnv64:{:016x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::types::{AgentMode, ApprovalMode, ClarificationMode};
    use crate::services::global_config::WorkerDefinition;

    fn worker_definition() -> WorkerDefinition {
        WorkerDefinition {
            name: "worker".to_string(),
            display_name: "Worker".to_string(),
            system_prompt: "prompt".to_string(),
            mode: AgentMode::Planning,
            approval_mode: ApprovalMode::Auto,
            clarification_mode: ClarificationMode::HeadlessDefer,
            capability_preset: CapabilityPreset::ReadOnlyReviewer,
            allow_delegate: false,
            allow_skill_activation: true,
            hidden_tools: vec!["review_check".to_string()],
            forced_tools: vec!["todowrite".to_string()],
            max_rounds: Some(DEFAULT_WORKER_MAX_ROUNDS),
            max_tool_calls: Some(DEFAULT_WORKER_MAX_TOOL_CALLS),
            model: Some("gpt-test".to_string()),
        }
    }

    #[test]
    fn worker_definition_maps_to_agent_profile() {
        let agent_profile = agent_profile_from_definition(&worker_definition());

        assert_eq!(agent_profile.mode, AgentMode::Planning);
        assert_eq!(agent_profile.approval_mode, ApprovalMode::Auto);
        assert_eq!(
            agent_profile.clarification_mode,
            ClarificationMode::HeadlessDefer
        );
        assert_eq!(
            agent_profile.capability_preset,
            CapabilityPreset::ReadOnlyReviewer
        );
        assert!(agent_profile.allow_skill_activation);
        assert_eq!(agent_profile.hidden_tools, vec!["review_check".to_string()]);
        assert_eq!(agent_profile.forced_tools, vec!["todowrite".to_string()]);
    }

    #[test]
    fn agent_profile_summary_carries_capability_fields() {
        let summary = AgentProfileSummary::from_agent_profile(&agent_profile_from_definition(
            &worker_definition(),
        ));

        assert_eq!(summary.mode, AgentMode::Planning);
        assert_eq!(
            summary.capability_preset,
            CapabilityPreset::ReadOnlyReviewer
        );
        assert!(summary.allow_skill_activation);
        assert!(!summary.profile_hash.is_empty());
    }

    #[test]
    fn builtin_general_worker_enables_skill_activation() {
        let profile = builtin_general_worker_profile();
        let policy = profile.capability_policy();

        assert_eq!(profile.capability_preset, CapabilityPreset::HeadlessWriter);
        assert!(policy.allow_skill_activation);
    }
}
