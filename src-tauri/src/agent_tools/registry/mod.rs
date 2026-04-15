//! Tool Registry - Unified tool manifest and discovery
//!
//! Uses the ToolDefinition trait from definition.rs.
//! Each tool is registered as a static entry with manifest metadata.

use serde::Serialize;

use crate::agent_engine::exposure_policy::{ExposureContext, SessionSource};
use crate::agent_engine::types::{AgentMode, ApprovalMode};
use crate::agent_tools::contracts::ToolDomain;
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext,
};

mod context_ops;
mod draft;
mod inspiration;
mod knowledge;
mod review;
mod structure;
mod utility;
mod workspace;

use context_ops::{CONTEXT_READ_TOOL, CONTEXT_SEARCH_TOOL};
use draft::DRAFT_WRITE_TOOL;
use inspiration::{INSPIRATION_CONSENSUS_PATCH_TOOL, INSPIRATION_OPEN_QUESTIONS_PATCH_TOOL};
use knowledge::{KNOWLEDGE_READ_TOOL, KNOWLEDGE_WRITE_TOOL};
use review::REVIEW_CHECK_TOOL;
use structure::STRUCTURE_EDIT_TOOL;
use utility::{ASKUSER_TOOL, SKILL_TOOL, TODOWRITE_TOOL};
use workspace::WORKSPACE_MAP_TOOL;

const DISALLOWED_PROVIDER_SCHEMA_KEYWORDS: &[&str] = &["oneOf", "anyOf", "allOf", "not", "const"];
const INSPIRATION_SESSION_ALLOWED_TOOLS: &[&str] = &[
    "inspiration_consensus_patch",
    "inspiration_open_questions_patch",
];

// ── Registry ──

static TOOL_REGISTRY: &[&dyn ToolDefinition] = &[
    &WORKSPACE_MAP_TOOL,
    &CONTEXT_READ_TOOL,
    &CONTEXT_SEARCH_TOOL,
    &KNOWLEDGE_READ_TOOL,
    &KNOWLEDGE_WRITE_TOOL,
    &DRAFT_WRITE_TOOL,
    &STRUCTURE_EDIT_TOOL,
    &REVIEW_CHECK_TOOL,
    &INSPIRATION_CONSENSUS_PATCH_TOOL,
    &INSPIRATION_OPEN_QUESTIONS_PATCH_TOOL,
    &ASKUSER_TOOL,
    &SKILL_TOOL,
    &TODOWRITE_TOOL,
];

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolLayer {
    CoreResource,
    DerivedView,
    SessionControl,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSafeRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolSchemaSkipDiagnostic {
    pub tool_name: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolHiddenDiagnostic {
    pub tool_name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ToolSchemaBuildReport {
    pub tools: Vec<serde_json::Value>,
    pub exposed_tools: Vec<String>,
    pub hidden_tools: Vec<ToolHiddenDiagnostic>,
    pub skipped_tools: Vec<ToolSchemaSkipDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolInventoryEntry {
    pub tool_name: String,
    pub manifest_id: String,
    pub llm_name: String,
    pub layer: ToolLayer,
    pub risk_level: crate::agent_tools::contracts::RiskLevel,
    pub confirmation: crate::agent_tools::contracts::ConfirmationPolicy,
    pub idempotency: crate::agent_tools::contracts::IdempotencyPolicy,
    pub parallel_safe: bool,
    pub externally_handled: bool,
    pub visible_in_writing_mode: bool,
    pub visible_in_planning_mode: bool,
    pub provider_safe: bool,
    pub provider_safe_risk: ProviderSafeRisk,
    pub schema_property_count: usize,
    pub schema_max_depth: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_schema_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSchemaInventory {
    pub total_tools: usize,
    pub provider_safe_tools: usize,
    pub tools: Vec<ToolInventoryEntry>,
}

/// Get the manifest for a tool by its LLM name.
pub fn get_manifest(tool_name: &str) -> Option<ToolManifest> {
    TOOL_REGISTRY
        .iter()
        .find(|t| t.name() == tool_name)
        .map(|t| t.manifest())
}

/// Get the LLM-facing description for a tool by its name.
pub fn get_description(tool_name: &str) -> Option<&'static str> {
    TOOL_REGISTRY
        .iter()
        .find(|t| t.name() == tool_name)
        .map(|t| t.description())
        .filter(|d| !d.is_empty())
}

/// Get the schema for a tool by its name under the provided context.
pub fn get_schema(tool_name: &str, context: &ToolSchemaContext) -> Option<serde_json::Value> {
    if tool_name == "askuser" && !context.clarification_mode.exposes_askuser() {
        return None;
    }

    TOOL_REGISTRY
        .iter()
        .find(|t| t.name() == tool_name)
        .and_then(|t| t.schema(context))
}

fn tool_layer(tool_name: &str) -> ToolLayer {
    match tool_name {
        "workspace_map" | "context_read" | "context_search" | "knowledge_read"
        | "knowledge_write" | "draft_write" | "structure_edit" | "review_check" => {
            ToolLayer::CoreResource
        }
        "inspiration_consensus_patch" | "inspiration_open_questions_patch" => {
            ToolLayer::DerivedView
        }
        "askuser" | "todowrite" | "skill" => ToolLayer::SessionControl,
        _ => ToolLayer::CoreResource,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SchemaStats {
    property_count: usize,
    max_depth: usize,
}

fn analyze_schema(schema: &serde_json::Value) -> SchemaStats {
    fn walk(node: &serde_json::Value, depth: usize, stats: &mut SchemaStats) {
        stats.max_depth = stats.max_depth.max(depth);

        match node {
            serde_json::Value::Object(map) => {
                if let Some(properties) = map.get("properties").and_then(|value| value.as_object())
                {
                    stats.property_count += properties.len();
                }

                for value in map.values() {
                    walk(value, depth + 1, stats);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    walk(value, depth + 1, stats);
                }
            }
            _ => {}
        }
    }

    let mut stats = SchemaStats::default();
    walk(schema, 1, &mut stats);
    stats
}

fn classify_provider_safe_risk(
    schema: Option<&serde_json::Value>,
    error: Option<&str>,
) -> ProviderSafeRisk {
    if error.is_some() {
        return ProviderSafeRisk::High;
    }

    let Some(schema) = schema else {
        return ProviderSafeRisk::Low;
    };

    let stats = analyze_schema(schema);
    if stats.max_depth >= 5 || stats.property_count >= 12 {
        ProviderSafeRisk::Medium
    } else {
        ProviderSafeRisk::Low
    }
}

pub fn build_tool_schema_inventory(context: &ToolSchemaContext) -> ToolSchemaInventory {
    let tools = TOOL_REGISTRY
        .iter()
        .map(|tool| {
            let manifest = tool.manifest();
            let schema = tool.schema(context);
            let provider_error = schema
                .as_ref()
                .and_then(|value| validate_provider_parameters_schema(value).err());
            let stats = schema.as_ref().map(analyze_schema).unwrap_or_default();

            ToolInventoryEntry {
                tool_name: tool.name().to_string(),
                manifest_id: manifest.id.to_string(),
                llm_name: manifest.llm_name.to_string(),
                layer: tool_layer(tool.name()),
                risk_level: manifest.risk_level,
                confirmation: manifest.confirmation,
                idempotency: manifest.idempotency,
                parallel_safe: manifest.parallel_safe,
                externally_handled: tool.externally_handled(),
                visible_in_writing_mode: visible_tools_for_mode(AgentMode::Writing)
                    .contains(&tool.name()),
                visible_in_planning_mode: visible_tools_for_mode(AgentMode::Planning)
                    .contains(&tool.name()),
                provider_safe: provider_error.is_none(),
                provider_safe_risk: classify_provider_safe_risk(
                    schema.as_ref(),
                    provider_error.as_deref(),
                ),
                schema_property_count: stats.property_count,
                schema_max_depth: stats.max_depth,
                provider_schema_error: provider_error,
            }
        })
        .collect::<Vec<_>>();

    ToolSchemaInventory {
        total_tools: tools.len(),
        provider_safe_tools: tools.iter().filter(|tool| tool.provider_safe).count(),
        tools,
    }
}

fn validate_provider_parameters_schema(schema: &serde_json::Value) -> Result<(), String> {
    validate_provider_parameters_schema_node(schema, "$", true)
}

fn validate_provider_parameters_schema_node(
    node: &serde_json::Value,
    path: &str,
    is_root: bool,
) -> Result<(), String> {
    match node {
        serde_json::Value::Object(map) => {
            if is_root {
                let schema_type = map.get("type").and_then(|value| value.as_str());
                if schema_type != Some("object") {
                    return Err(format!("{path} must declare type='object'"));
                }

                if map.contains_key("enum") {
                    return Err(format!("{path} must not declare top-level enum"));
                }
            }

            for keyword in DISALLOWED_PROVIDER_SCHEMA_KEYWORDS {
                if map.contains_key(*keyword) {
                    return Err(format!("{path} uses unsupported keyword '{keyword}'"));
                }
            }

            for (key, value) in map {
                let child_path = format!("{path}.{key}");
                validate_provider_parameters_schema_node(value, &child_path, false)?;
            }

            Ok(())
        }
        serde_json::Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                let child_path = format!("{path}[{index}]");
                validate_provider_parameters_schema_node(value, &child_path, false)?;
            }

            Ok(())
        }
        _ => Ok(()),
    }
}

/// Build OpenAI-compatible tool schema list from registry + context.
pub fn build_openai_tool_schema_report(
    mode: AgentMode,
    context: &ToolSchemaContext,
) -> ToolSchemaBuildReport {
    let exposure = default_exposure_context(mode, context);
    build_openai_tool_schema_report_for_exposure(&exposure, context)
}

pub fn build_openai_tool_schema_report_for_exposure(
    exposure: &ExposureContext,
    context: &ToolSchemaContext,
) -> ToolSchemaBuildReport {
    let mut tools = Vec::new();
    let mut exposed_tools = Vec::new();
    let mut hidden_tools = Vec::new();
    let mut skipped_tools = Vec::new();

    for tool in TOOL_REGISTRY
        .iter()
        .filter(|tool| matches!(tool.manifest().domain, ToolDomain::Novel))
    {
        let manifest = tool.manifest();
        if let Some(reason) = tool_hidden_reason_for_exposure(tool.name(), &manifest, exposure) {
            hidden_tools.push(ToolHiddenDiagnostic {
                tool_name: tool.name().to_string(),
                reason,
            });
            continue;
        }

        let Some(parameters) = tool.schema(context) else {
            hidden_tools.push(ToolHiddenDiagnostic {
                tool_name: tool.name().to_string(),
                reason: "schema_unavailable".to_string(),
            });
            continue;
        };

        if let Err(error) = validate_provider_parameters_schema(&parameters) {
            tracing::warn!(
                target: "agent_tools",
                tool = tool.name(),
                error = %error,
                "skipping provider-incompatible tool schema"
            );
            skipped_tools.push(ToolSchemaSkipDiagnostic {
                tool_name: tool.name().to_string(),
                error,
            });
            continue;
        }

        exposed_tools.push(tool.name().to_string());
        tools.push(tool_schema_entry(
            tool.name(),
            tool.description(),
            parameters,
        ));
    }

    ToolSchemaBuildReport {
        tools,
        exposed_tools,
        hidden_tools,
        skipped_tools,
    }
}

pub fn build_openai_tool_schemas(
    mode: AgentMode,
    context: &ToolSchemaContext,
) -> Vec<serde_json::Value> {
    build_openai_tool_schema_report(mode, context).tools
}

/// Get all tool definitions.
pub fn get_all_definitions() -> &'static [&'static dyn ToolDefinition] {
    TOOL_REGISTRY
}

/// Get visible tool names for the model.
pub fn visible_tools_for_model() -> Vec<&'static str> {
    visible_tools_for_exposure(&default_exposure_context(
        AgentMode::Writing,
        &ToolSchemaContext::default(),
    ))
}

fn default_exposure_context(mode: AgentMode, context: &ToolSchemaContext) -> ExposureContext {
    ExposureContext::new(
        mode,
        ApprovalMode::ConfirmWrites,
        context.clarification_mode,
        SessionSource::UserInteractive,
        0,
        context.semantic_retrieval_enabled,
        None,
        None,
        crate::agent_engine::exposure_policy::CapabilityPolicy::default_for_mode(
            mode,
            context.clarification_mode,
        ),
    )
}

fn tool_schema_entry(
    tool_name: &str,
    description: &str,
    parameters: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": tool_name,
            "description": description,
            "parameters": parameters,
        }
    })
}

fn tool_hidden_reason_for_exposure(
    tool_name: &str,
    manifest: &ToolManifest,
    exposure: &ExposureContext,
) -> Option<String> {
    if let Some(reason) = tool_hidden_reason_for_hard_boundaries(tool_name, manifest, exposure) {
        return Some(reason);
    }

    if exposure.capability_policy.hides_tool(tool_name) {
        return Some("capability_policy:hidden_override".to_string());
    }

    if exposure.capability_policy.forces_tool(tool_name) {
        return None;
    }

    exposure
        .capability_policy
        .capability_denial_reason(manifest.capabilities)
}

fn tool_hidden_reason_for_hard_boundaries(
    tool_name: &str,
    manifest: &ToolManifest,
    exposure: &ExposureContext,
) -> Option<String> {
    if let Some(reason) = tool_hidden_reason_for_profile_boundary(tool_name, exposure) {
        return Some(reason);
    }

    if !session_source_allowed(manifest, exposure.session_source) {
        return Some(format!(
            "session_source:{}",
            exposure.session_source.as_str()
        ));
    }

    if tool_name == "askuser" && !exposure.clarification_mode.exposes_askuser() {
        return Some("clarification_mode:headless_defer".to_string());
    }

    if !tool_allowed_in_mode(manifest, exposure.mode) {
        return Some("mode:planning".to_string());
    }

    None
}

fn tool_hidden_reason_for_profile_boundary(
    tool_name: &str,
    exposure: &ExposureContext,
) -> Option<String> {
    let allowed_tools = match exposure.active_profile.as_deref() {
        Some(profile) if profile.eq_ignore_ascii_case("inspiration_session") => {
            Some(INSPIRATION_SESSION_ALLOWED_TOOLS)
        }
        _ => None,
    }?;

    if allowed_tools.iter().any(|allowed| *allowed == tool_name) {
        None
    } else {
        Some(format!(
            "profile_boundary:{}",
            exposure.active_profile.as_deref().unwrap_or("unknown")
        ))
    }
}

fn session_source_allowed(manifest: &ToolManifest, session_source: SessionSource) -> bool {
    if manifest.visibility.interactive_only
        && !matches!(session_source, SessionSource::UserInteractive)
    {
        return false;
    }

    match session_source {
        SessionSource::UserInteractive => true,
        SessionSource::Delegate => manifest.visibility.allow_in_delegate,
        SessionSource::WorkflowJob => manifest.visibility.allow_in_workflow_job,
        SessionSource::ReviewGate => manifest.visibility.allow_in_review_gate,
    }
}

fn tool_allowed_in_mode(manifest: &ToolManifest, mode: AgentMode) -> bool {
    match mode {
        AgentMode::Writing => true,
        AgentMode::Planning => !tool_requires_write_capability(manifest.capabilities),
    }
}

fn tool_requires_write_capability(capabilities: &[ToolCapability]) -> bool {
    capabilities.iter().any(|capability| {
        matches!(
            capability,
            ToolCapability::DraftWrite
                | ToolCapability::StructureWrite
                | ToolCapability::KnowledgeWrite
        )
    })
}

fn visible_tools_for_exposure(exposure: &ExposureContext) -> Vec<&'static str> {
    TOOL_REGISTRY
        .iter()
        .filter(|tool| matches!(tool.manifest().domain, ToolDomain::Novel))
        .filter(|tool| {
            let manifest = tool.manifest();
            tool_hidden_reason_for_exposure(tool.name(), &manifest, exposure).is_none()
        })
        .map(|tool| tool.name())
        .collect()
}

/// Get visible tool names filtered by agent mode.
pub fn visible_tools_for_mode(mode: AgentMode) -> Vec<&'static str> {
    visible_tools_for_exposure(&default_exposure_context(
        mode,
        &ToolSchemaContext::default(),
    ))
}
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn test_all_tools_have_manifests() {
        for tool in [
            "workspace_map",
            "context_read",
            "context_search",
            "knowledge_read",
            "knowledge_write",
            "draft_write",
            "structure_edit",
            "review_check",
            "inspiration_consensus_patch",
            "inspiration_open_questions_patch",
            "askuser",
            "skill",
            "todowrite",
        ] {
            assert!(
                get_manifest(tool).is_some(),
                "manifest should exist for tool '{}'",
                tool
            );
        }
    }

    #[test]
    fn test_all_tools_have_descriptions() {
        for tool_def in TOOL_REGISTRY.iter() {
            let desc = tool_def.description();
            assert!(
                !desc.is_empty(),
                "description should not be empty for tool '{}'",
                tool_def.name()
            );
        }
    }

    #[test]
    fn test_context_read_description_mentions_ref_and_budgeting() {
        let desc = get_description("context_read").expect("context_read description");
        assert!(
            desc.contains("<kind>:<project_relative_path>"),
            "context_read should document the ref form"
        );
        assert!(
            desc.contains("budget") || desc.contains("budgeted"),
            "context_read should mention budgeted output"
        );
    }

    #[test]
    fn test_draft_write_description_does_not_mention_legacy_parameters() {
        let desc = get_description("draft_write").expect("draft_write description");
        let forbidden = [
            "snapshot".to_string() + "_id",
            "base".to_string() + "_revision",
            "o".to_string() + "ps",
        ];
        for forbidden in forbidden {
            assert!(
                !desc.contains(&forbidden),
                "draft_write description must not mention legacy field '{forbidden}'"
            );
        }
    }

    #[test]
    fn test_parallel_safe_tools_have_performance_tip() {
        for tool_def in TOOL_REGISTRY.iter() {
            let manifest = tool_def.manifest();
            if manifest.parallel_safe {
                let desc = tool_def.description();
                assert!(
                    desc.contains("PERFORMANCE TIP") || desc.contains("parallel"),
                    "parallel_safe tool '{}' should mention parallel usage in description",
                    tool_def.name()
                );
            }
        }
    }

    #[test]
    fn test_all_visible_tools_have_schemas_with_default_context() {
        let context = ToolSchemaContext::default();

        for name in visible_tools_for_mode(AgentMode::Writing) {
            assert!(
                get_schema(name, &context).is_some(),
                "tool '{}' should have a schema",
                name
            );
        }
    }

    #[test]
    fn test_validate_provider_parameters_schema_rejects_top_level_combinators() {
        let schema = serde_json::json!({
            "type": "object",
            "oneOf": [{ "type": "object" }]
        });

        let error =
            validate_provider_parameters_schema(&schema).expect_err("schema should be rejected");
        assert!(error.contains("unsupported keyword 'oneOf'"));
    }

    #[test]
    fn test_validate_provider_parameters_schema_rejects_nested_const() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "const": "chapter_content"
                }
            }
        });

        let error =
            validate_provider_parameters_schema(&schema).expect_err("schema should be rejected");
        assert!(error.contains("unsupported keyword 'const'"));
    }

    #[test]
    fn test_all_built_tool_schemas_are_provider_compatible() {
        let context = ToolSchemaContext::default();
        let tools = build_openai_tool_schemas(AgentMode::Writing, &context);

        assert_eq!(
            tools.len(),
            10,
            "all writing-mode tools should build successfully"
        );

        for tool in tools {
            let parameters = tool
                .get("function")
                .and_then(|value| value.get("parameters"))
                .expect("function parameters");
            validate_provider_parameters_schema(parameters)
                .expect("tool schema should be provider compatible");
        }
    }

    #[test]
    fn test_build_tool_schema_report_tracks_exposed_and_skipped_tools() {
        let context = ToolSchemaContext::default();
        let report = build_openai_tool_schema_report(AgentMode::Writing, &context);

        assert_eq!(report.tools.len(), 10);
        assert_eq!(report.exposed_tools.len(), 10);
        assert!(report
            .hidden_tools
            .iter()
            .any(|tool| tool.tool_name == "skill"));
        assert!(report.skipped_tools.is_empty());
    }

    #[test]
    fn test_build_tool_schema_inventory_covers_registered_tools() {
        let inventory = build_tool_schema_inventory(&ToolSchemaContext::default());

        assert_eq!(inventory.total_tools, 13);
        assert_eq!(inventory.provider_safe_tools, 13);
        assert_eq!(inventory.tools.len(), 13);
        assert!(inventory
            .tools
            .iter()
            .any(|tool| tool.tool_name == "draft_write"
                && tool.provider_safe_risk == ProviderSafeRisk::Medium));
    }

    #[test]
    fn test_context_search_schema_keyword_only_when_semantic_disabled() {
        let context = ToolSchemaContext::default();
        let schema = get_schema("context_search", &context).expect("context_search schema");
        let modes = schema
            .get("properties")
            .and_then(|p| p.get("mode"))
            .and_then(|m| m.get("enum"))
            .and_then(|e| e.as_array())
            .expect("mode enum");
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].as_str(), Some("keyword"));
    }

    #[test]
    fn test_context_search_schema_exposes_semantic_modes_when_enabled() {
        let context = ToolSchemaContext {
            semantic_retrieval_enabled: true,
            ..ToolSchemaContext::default()
        };
        let schema = get_schema("context_search", &context).expect("context_search schema");
        let modes = schema
            .get("properties")
            .and_then(|p| p.get("mode"))
            .and_then(|m| m.get("enum"))
            .and_then(|e| e.as_array())
            .expect("mode enum");
        assert_eq!(modes.len(), 3);
        assert!(modes.iter().any(|v| v.as_str() == Some("semantic")));
        assert!(modes.iter().any(|v| v.as_str() == Some("hybrid")));
    }

    #[test]
    fn test_headless_context_hides_askuser_schema() {
        let context = ToolSchemaContext {
            clarification_mode: crate::agent_engine::types::ClarificationMode::HeadlessDefer,
            ..ToolSchemaContext::default()
        };
        assert!(get_schema("askuser", &context).is_none());
    }

    #[test]
    fn test_inspiration_session_profile_boundary_only_exposes_patch_tools() {
        let mut policy = crate::agent_engine::exposure_policy::CapabilityPolicy::new(
            crate::agent_engine::exposure_policy::CapabilityPreset::MainPlanning,
        );
        policy.forced_tools = INSPIRATION_SESSION_ALLOWED_TOOLS
            .iter()
            .map(|tool| tool.to_string())
            .collect();
        let exposure = ExposureContext::new(
            AgentMode::Planning,
            ApprovalMode::ConfirmWrites,
            crate::agent_engine::types::ClarificationMode::Interactive,
            SessionSource::UserInteractive,
            0,
            false,
            None,
            Some("inspiration_session".to_string()),
            policy,
        );

        let report =
            build_openai_tool_schema_report_for_exposure(&exposure, &ToolSchemaContext::default());

        assert_eq!(
            report.exposed_tools,
            INSPIRATION_SESSION_ALLOWED_TOOLS
                .iter()
                .map(|tool| tool.to_string())
                .collect::<Vec<_>>()
        );
        assert!(report.hidden_tools.iter().any(|tool| {
            tool.tool_name == "askuser" && tool.reason == "profile_boundary:inspiration_session"
        }));
    }

    #[test]
    fn test_skill_schema_uses_context_skill_enum() {
        let context = ToolSchemaContext {
            available_skills: vec!["story-architect".to_string(), "plot-audit".to_string()],
            ..ToolSchemaContext::default()
        };

        let schema = get_schema("skill", &context).expect("skill schema");
        let enum_values = schema
            .get("properties")
            .and_then(|p| p.get("skill"))
            .and_then(|s| s.get("enum"))
            .and_then(|e| e.as_array())
            .expect("skill enum");

        assert!(enum_values
            .iter()
            .any(|v| v.as_str() == Some("story-architect")));
        assert!(enum_values.iter().any(|v| v.as_str() == Some("plot-audit")));
    }

    #[test]
    fn test_visible_tools_for_model_returns_all_novel_tools() {
        let expected: BTreeSet<&'static str> = visible_tools_for_mode(AgentMode::Writing)
            .into_iter()
            .collect();

        let actual: BTreeSet<&'static str> = visible_tools_for_model().into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_readonly_tools_are_parallel_safe() {
        for name in [
            "workspace_map",
            "context_read",
            "context_search",
            "knowledge_read",
            "review_check",
        ] {
            let manifest = get_manifest(name).expect("manifest should exist");
            assert!(manifest.parallel_safe, "{} should be parallel_safe", name);
        }
    }

    #[test]
    fn test_registry_has_13_tools() {
        assert_eq!(TOOL_REGISTRY.len(), 13);
    }

    #[test]
    fn test_externally_handled_tools() {
        for name in ["askuser", "skill"] {
            let def = TOOL_REGISTRY.iter().find(|t| t.name() == name).unwrap();
            assert!(
                def.externally_handled(),
                "{} should be externally handled",
                name
            );
        }
        for name in [
            "workspace_map",
            "context_read",
            "context_search",
            "knowledge_read",
            "knowledge_write",
            "draft_write",
            "structure_edit",
            "review_check",
            "inspiration_consensus_patch",
            "inspiration_open_questions_patch",
            "todowrite",
        ] {
            let def = TOOL_REGISTRY.iter().find(|t| t.name() == name).unwrap();
            assert!(
                !def.externally_handled(),
                "{} should NOT be externally handled",
                name
            );
        }
    }

    #[test]
    fn test_planning_mode_hides_write_tools() {
        let visible = visible_tools_for_mode(AgentMode::Planning);
        assert!(
            !visible.contains(&"draft_write"),
            "Planning should hide draft_write"
        );
        assert!(
            !visible.contains(&"structure_edit"),
            "Planning should hide structure_edit"
        );
        assert!(
            !visible.contains(&"knowledge_write"),
            "Planning should hide knowledge_write"
        );
        assert!(visible.contains(&"workspace_map"));
        assert!(visible.contains(&"context_read"));
        assert!(visible.contains(&"context_search"));
        assert!(visible.contains(&"knowledge_read"));
        assert!(visible.contains(&"review_check"));
        assert!(visible.contains(&"askuser"));
        assert!(visible.contains(&"todowrite"));
        assert!(!visible.contains(&"skill"));
    }

    #[test]
    fn test_writing_mode_includes_all_tools() {
        let visible = visible_tools_for_mode(AgentMode::Writing);
        assert_eq!(visible.len(), 10);
        assert!(visible.contains(&"draft_write"));
        assert!(visible.contains(&"knowledge_write"));
        assert!(visible.contains(&"structure_edit"));
        assert!(visible.contains(&"review_check"));
        assert!(visible.contains(&"askuser"));
        assert!(visible.contains(&"todowrite"));
        assert!(!visible.contains(&"skill"));
        assert!(!visible.contains(&"inspiration_consensus_patch"));
    }

    #[test]
    fn test_tool_schemas_do_not_expose_unimplemented_per_call_timeout_ms() {
        let context = ToolSchemaContext::default();
        for tool_name in [
            "workspace_map",
            "context_read",
            "context_search",
            "knowledge_read",
            "knowledge_write",
            "draft_write",
            "structure_edit",
        ] {
            let schema = get_schema(tool_name, &context).expect("schema");
            let properties = schema
                .get("properties")
                .and_then(|value| value.as_object())
                .expect("schema properties");
            assert!(
                !properties.contains_key("timeout_ms"),
                "{tool_name} should not expose timeout_ms until per-call timeout negotiation is implemented"
            );
        }
    }

    #[test]
    fn test_structure_edit_schema_hides_unimplemented_knowledge_item_node_type() {
        let schema = get_schema("structure_edit", &ToolSchemaContext::default())
            .expect("structure_edit schema");
        let node_type_values = schema
            .get("properties")
            .and_then(|value| value.get("node_type"))
            .and_then(|value| value.get("enum"))
            .and_then(|value| value.as_array())
            .expect("node_type enum");

        assert!(node_type_values
            .iter()
            .all(|value| value.as_str() != Some("knowledge_item")));
        assert!(node_type_values
            .iter()
            .any(|value| value.as_str() == Some("volume")));
        assert!(node_type_values
            .iter()
            .any(|value| value.as_str() == Some("chapter")));
    }
}
