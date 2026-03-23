use serde::Serialize;
use serde_json::json;

use crate::agent_engine::exposure_policy::{
    CapabilityPolicy, ExposureContext, ExposurePolicySummary, SessionSource,
};
use crate::agent_engine::messages::ConversationState;
use crate::agent_engine::tool_schemas::{build_tool_schema_bundle_for_exposure, BuiltToolSchemas};
use crate::agent_engine::types::LoopConfig;
use crate::agent_tools::registry::{ToolHiddenDiagnostic, ToolSchemaSkipDiagnostic};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ToolExposureTelemetry {
    pub policy_source: String,
    pub exposure_reason: String,
    pub capability_preset: String,
    pub tool_package: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_from: Option<String>,
    pub policy_summary: ExposurePolicySummary,
    pub exposed_tools: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hidden_tools: Vec<ToolHiddenDiagnostic>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped_tools: Vec<ToolSchemaSkipDiagnostic>,
}

impl ToolExposureTelemetry {
    pub fn to_payload(&self) -> serde_json::Value {
        json!({
            "policy_source": self.policy_source,
            "policy_summary": self.policy_summary,
            "exposure_reason": self.exposure_reason,
            "capability_preset": self.capability_preset,
            "tool_package": self.tool_package,
            "fallback_from": self.fallback_from,
            "exposed_tools": self.exposed_tools,
            "hidden_tools": self.hidden_tools,
            "skipped_tools": self.skipped_tools,
        })
    }

    fn capability_policy(exposure: &ExposureContext, bundle: &BuiltToolSchemas) -> Self {
        let preset = exposure.capability_policy.preset.as_str().to_string();
        let (tool_package, fallback_from) = resolve_tool_package(exposure);
        Self {
            policy_source: "capability_policy".to_string(),
            exposure_reason: format!("capability_policy.{preset}"),
            capability_preset: preset,
            tool_package,
            fallback_from,
            policy_summary: exposure.policy_summary(),
            exposed_tools: bundle.exposed_tools.clone(),
            hidden_tools: bundle.hidden_tools.clone(),
            skipped_tools: bundle.skipped_tools.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedToolExposure {
    pub bundle: BuiltToolSchemas,
    pub telemetry: ToolExposureTelemetry,
}

pub(crate) fn resolve_turn_tool_exposure(
    _state: &ConversationState,
    config: &LoopConfig,
    active_chapter_path: Option<&str>,
    semantic_retrieval_enabled: bool,
) -> ResolvedToolExposure {
    let exposure = ExposureContext::new(
        config.capability_mode,
        config.approval_mode,
        config.clarification_mode,
        SessionSource::UserInteractive,
        0,
        semantic_retrieval_enabled,
        active_chapter_path.map(|path| path.to_string()),
        None,
        CapabilityPolicy::default_for_mode(config.capability_mode, config.clarification_mode),
    );

    resolve_turn_tool_exposure_with_context(config, active_chapter_path, exposure)
}

pub(crate) fn resolve_turn_tool_exposure_with_context(
    config: &LoopConfig,
    active_chapter_path: Option<&str>,
    exposure: ExposureContext,
) -> ResolvedToolExposure {
    let exposure = ExposureContext::new(
        config.capability_mode,
        config.approval_mode,
        config.clarification_mode,
        exposure.session_source,
        exposure.delegate_depth,
        exposure.semantic_retrieval_enabled,
        exposure
            .active_chapter_path
            .clone()
            .or_else(|| active_chapter_path.map(|path| path.to_string())),
        exposure.active_profile.clone(),
        exposure.capability_policy.clone(),
    );

    let bundle = build_tool_schema_bundle_for_exposure(&exposure);
    let telemetry = ToolExposureTelemetry::capability_policy(&exposure, &bundle);
    ResolvedToolExposure { bundle, telemetry }
}

fn resolve_tool_package(exposure: &ExposureContext) -> (String, Option<String>) {
    let base_package = match exposure.session_source {
        SessionSource::UserInteractive => "main_agent_core",
        SessionSource::Delegate => "delegate_agent_core",
        SessionSource::WorkflowJob => "workflow_job_core",
        SessionSource::ReviewGate => "review_gate_core",
    };

    let profile_package = exposure
        .active_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .map(|profile| profile.to_ascii_lowercase());

    match profile_package {
        Some(profile) if profile == base_package => (profile, None),
        Some(profile) => (profile, Some(base_package.to_string())),
        None => (base_package.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::messages::AgentMessage;
    use crate::agent_engine::types::{AgentMode, ApprovalMode, ClarificationMode};

    fn config(mode: AgentMode, clarification_mode: ClarificationMode) -> LoopConfig {
        LoopConfig {
            capability_mode: mode,
            approval_mode: ApprovalMode::ConfirmWrites,
            clarification_mode,
            ..LoopConfig::default()
        }
    }

    fn state() -> ConversationState {
        let mut state = ConversationState::new("sess_tool_exposure".to_string());
        state
            .messages
            .push(AgentMessage::user("please help".to_string()));
        state
    }

    #[test]
    fn writing_mode_uses_main_interactive_capability_policy() {
        let resolved = resolve_turn_tool_exposure(
            &state(),
            &config(AgentMode::Writing, ClarificationMode::Interactive),
            None,
            false,
        );

        assert_eq!(resolved.telemetry.policy_source, "capability_policy");
        assert_eq!(resolved.telemetry.capability_preset, "main_interactive");
        assert!(resolved
            .telemetry
            .exposed_tools
            .contains(&"draft_write".to_string()));
        assert!(resolved
            .telemetry
            .exposed_tools
            .contains(&"structure_edit".to_string()));
        assert!(resolved
            .telemetry
            .hidden_tools
            .iter()
            .any(|tool| tool.tool_name == "skill"
                && tool.reason == "capability_policy:skill_disabled"));
    }

    #[test]
    fn planning_mode_hides_write_tools_with_reasons() {
        let resolved = resolve_turn_tool_exposure(
            &state(),
            &config(AgentMode::Planning, ClarificationMode::Interactive),
            None,
            false,
        );

        assert!(!resolved
            .telemetry
            .exposed_tools
            .contains(&"draft_write".to_string()));
        assert!(resolved
            .telemetry
            .hidden_tools
            .iter()
            .any(|tool| tool.tool_name == "draft_write" && tool.reason == "mode:planning"));
        assert!(resolved
            .telemetry
            .exposed_tools
            .contains(&"askuser".to_string()));
    }

    #[test]
    fn headless_mode_hides_askuser() {
        let resolved = resolve_turn_tool_exposure(
            &state(),
            &config(AgentMode::Writing, ClarificationMode::HeadlessDefer),
            None,
            false,
        );

        assert!(!resolved
            .telemetry
            .exposed_tools
            .contains(&"askuser".to_string()));
        assert!(resolved
            .telemetry
            .hidden_tools
            .iter()
            .any(|tool| tool.tool_name == "askuser"
                && tool.reason == "clarification_mode:headless_defer"));
    }

    #[test]
    fn delegate_source_hides_main_session_only_tools() {
        let exposure = ExposureContext::new(
            AgentMode::Writing,
            ApprovalMode::ConfirmWrites,
            ClarificationMode::Interactive,
            SessionSource::Delegate,
            1,
            false,
            None,
            Some("delegate_writer".to_string()),
            CapabilityPolicy::new(
                crate::agent_engine::exposure_policy::CapabilityPreset::HeadlessWriter,
            ),
        );

        let resolved = resolve_turn_tool_exposure_with_context(
            &config(AgentMode::Writing, ClarificationMode::Interactive),
            None,
            exposure,
        );

        assert!(!resolved
            .telemetry
            .exposed_tools
            .contains(&"askuser".to_string()));
        assert!(resolved
            .telemetry
            .hidden_tools
            .iter()
            .any(|tool| tool.tool_name == "askuser" && tool.reason == "session_source:delegate"));
    }

    #[test]
    fn forced_tool_overrides_can_expose_inspiration_tools_without_profile_boundary() {
        let mut policy = CapabilityPolicy::new(
            crate::agent_engine::exposure_policy::CapabilityPreset::MainPlanning,
        );
        policy.allow_delegate = false;
        policy.forced_tools = vec![
            "inspiration_consensus_patch".to_string(),
            "inspiration_open_questions_patch".to_string(),
        ];
        let exposure = ExposureContext::new(
            AgentMode::Planning,
            ApprovalMode::ConfirmWrites,
            ClarificationMode::Interactive,
            SessionSource::UserInteractive,
            0,
            false,
            None,
            Some("inspiration".to_string()),
            policy,
        );

        let resolved = resolve_turn_tool_exposure_with_context(
            &config(AgentMode::Planning, ClarificationMode::Interactive),
            None,
            exposure,
        );

        assert_eq!(resolved.telemetry.policy_source, "capability_policy");
        assert!(resolved
            .telemetry
            .exposed_tools
            .contains(&"inspiration_consensus_patch".to_string()));
        assert!(resolved
            .telemetry
            .exposed_tools
            .contains(&"inspiration_open_questions_patch".to_string()));
    }

    #[test]
    fn telemetry_uses_main_agent_core_package_for_default_turns() {
        let resolved = resolve_turn_tool_exposure(
            &state(),
            &config(AgentMode::Writing, ClarificationMode::Interactive),
            None,
            false,
        );

        assert_eq!(resolved.telemetry.tool_package, "main_agent_core");
        assert!(resolved.telemetry.fallback_from.is_none());
    }

    #[test]
    fn telemetry_tracks_profile_boundary_fallback_chain() {
        let mut policy = CapabilityPolicy::new(
            crate::agent_engine::exposure_policy::CapabilityPreset::MainPlanning,
        );
        policy.forced_tools = vec![
            "inspiration_consensus_patch".to_string(),
            "inspiration_open_questions_patch".to_string(),
        ];
        let exposure = ExposureContext::new(
            AgentMode::Planning,
            ApprovalMode::ConfirmWrites,
            ClarificationMode::Interactive,
            SessionSource::UserInteractive,
            0,
            false,
            None,
            Some("inspiration_session".to_string()),
            policy,
        );

        let resolved = resolve_turn_tool_exposure_with_context(
            &config(AgentMode::Planning, ClarificationMode::Interactive),
            None,
            exposure,
        );

        assert_eq!(resolved.telemetry.tool_package, "inspiration_session");
        assert_eq!(
            resolved.telemetry.fallback_from.as_deref(),
            Some("main_agent_core")
        );
    }
}
