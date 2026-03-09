//! Tool Registry - Unified tool manifest and discovery
//!
//! Uses the ToolDefinition trait from definition.rs.
//! Each tool is registered as a static entry with manifest metadata.

use serde::Serialize;

use crate::agent_engine::types::AgentMode;
use crate::agent_tools::contracts::ToolDomain;
use crate::agent_tools::definition::{ToolDefinition, ToolManifest, ToolSchemaContext};

mod context;
mod discovery;
mod utility;
mod writing;

use context::{CHARACTER_SHEET_TOOL, OUTLINE_TOOL, SEARCH_KNOWLEDGE_TOOL};
use discovery::{GREP_TOOL, LS_TOOL};
use utility::{ASKUSER_TOOL, SKILL_TOOL, TODOWRITE_TOOL};
use writing::{CREATE_TOOL, DELETE_TOOL, EDIT_TOOL, MOVE_TOOL, READ_TOOL};

const DISALLOWED_PROVIDER_SCHEMA_KEYWORDS: &[&str] = &["oneOf", "anyOf", "allOf", "not", "const"];

// ── Registry ──

static TOOL_REGISTRY: &[&dyn ToolDefinition] = &[
    &READ_TOOL,
    &EDIT_TOOL,
    &CREATE_TOOL,
    &DELETE_TOOL,
    &MOVE_TOOL,
    &LS_TOOL,
    &GREP_TOOL,
    &ASKUSER_TOOL,
    &SKILL_TOOL,
    &TODOWRITE_TOOL,
    &OUTLINE_TOOL,
    &CHARACTER_SHEET_TOOL,
    &SEARCH_KNOWLEDGE_TOOL,
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ToolSchemaBuildReport {
    pub tools: Vec<serde_json::Value>,
    pub exposed_tools: Vec<String>,
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
        "read" | "edit" | "create" | "delete" | "move" | "ls" | "grep" => ToolLayer::CoreResource,
        "outline" | "character_sheet" | "search_knowledge" => ToolLayer::DerivedView,
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
    let visible = visible_tools_for_mode(mode);
    let mut tools = Vec::new();
    let mut exposed_tools = Vec::new();
    let mut skipped_tools = Vec::new();

    for tool in TOOL_REGISTRY
        .iter()
        .filter(|tool| visible.contains(&tool.name()))
        .filter(|tool| !(tool.name() == "askuser" && !context.clarification_mode.exposes_askuser()))
    {
        let Some(parameters) = tool.schema(context) else {
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
        tools.push(serde_json::json!({
            "type": "function",
            "function": {
                "name": tool.name(),
                "description": tool.description(),
                "parameters": parameters,
            }
        }));
    }

    ToolSchemaBuildReport {
        tools,
        exposed_tools,
        skipped_tools,
    }
}

pub fn build_openai_tool_schemas(
    mode: AgentMode,
    context: &ToolSchemaContext,
) -> Vec<serde_json::Value> {
    build_openai_tool_schema_report(mode, context).tools
}

/// Build OpenAI-compatible schemas and then apply a whitelist filter.
pub fn build_filtered_openai_tool_schema_report(
    whitelist: &[String],
    mode: AgentMode,
    context: &ToolSchemaContext,
) -> ToolSchemaBuildReport {
    let report = build_openai_tool_schema_report(mode, context);
    let tools = report
        .tools
        .into_iter()
        .filter(|tool| {
            tool.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|name| whitelist.iter().any(|w| w == name))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let exposed_tools = report
        .exposed_tools
        .into_iter()
        .filter(|name| whitelist.iter().any(|allowed| allowed == name))
        .collect::<Vec<_>>();
    let skipped_tools = report
        .skipped_tools
        .into_iter()
        .filter(|tool| whitelist.iter().any(|allowed| allowed == &tool.tool_name))
        .collect::<Vec<_>>();

    ToolSchemaBuildReport {
        tools,
        exposed_tools,
        skipped_tools,
    }
}

pub fn build_filtered_openai_tool_schemas(
    whitelist: &[String],
    mode: AgentMode,
    context: &ToolSchemaContext,
) -> Vec<serde_json::Value> {
    build_filtered_openai_tool_schema_report(whitelist, mode, context).tools
}

/// Get all tool definitions.
pub fn get_all_definitions() -> &'static [&'static dyn ToolDefinition] {
    TOOL_REGISTRY
}

/// Get visible tool names for the model.
pub fn visible_tools_for_model() -> Vec<&'static str> {
    TOOL_REGISTRY
        .iter()
        .filter(|t| matches!(t.manifest().domain, ToolDomain::Novel))
        .map(|t| t.name())
        .collect()
}

/// Tools hidden in Planning mode (read-only exploration).
const PLANNING_HIDDEN: &[&str] = &["edit", "create", "delete", "move"];

/// Get visible tool names filtered by agent mode.
pub fn visible_tools_for_mode(mode: AgentMode) -> Vec<&'static str> {
    TOOL_REGISTRY
        .iter()
        .filter(|t| matches!(t.manifest().domain, ToolDomain::Novel))
        .filter(|t| match mode {
            AgentMode::Writing => true,
            AgentMode::Planning => !PLANNING_HIDDEN.contains(&t.name()),
        })
        .map(|t| t.name())
        .collect()
}
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_all_tools_have_manifests() {
        for tool in [
            "read",
            "edit",
            "create",
            "delete",
            "move",
            "ls",
            "grep",
            "askuser",
            "skill",
            "todowrite",
            "outline",
            "character_sheet",
            "search_knowledge",
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
    fn test_edit_description_references_read() {
        let desc = get_description("edit").expect("edit description");
        assert!(
            desc.contains("read"),
            "edit description should reference 'read' tool"
        );
    }

    #[test]
    fn test_read_description_references_edit() {
        let desc = get_description("read").expect("read description");
        assert!(
            desc.contains("edit") || desc.contains("base_revision"),
            "read description should reference edit workflow"
        );
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
            13,
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

        assert_eq!(report.tools.len(), 13);
        assert_eq!(report.exposed_tools.len(), 13);
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
            .any(|tool| tool.tool_name == "edit"
                && tool.provider_safe_risk == ProviderSafeRisk::Medium));
    }

    #[test]
    fn test_registry_tools_have_dispatch_or_external_handler() {
        for tool in TOOL_REGISTRY {
            assert!(
                crate::agent_engine::tool_dispatch::dispatch_supports_tool(tool.name())
                    || tool.externally_handled(),
                "tool '{}' must either dispatch or be externally handled",
                tool.name()
            );
        }
    }

    #[test]
    fn test_tool_schema_inventory_matches_snapshot() {
        let inventory = build_tool_schema_inventory(&ToolSchemaContext::default());
        let actual = serde_json::to_value(inventory).expect("inventory should serialize");
        let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("schemas")
            .join("agent-tool-schema-inventory.json");
        let expected = std::fs::read_to_string(&snapshot_path).unwrap_or_else(|error| {
            panic!(
                "failed to read snapshot {}: {error}",
                snapshot_path.display()
            )
        });
        let expected: serde_json::Value =
            serde_json::from_str(&expected).expect("snapshot should be valid json");

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_grep_schema_keyword_only_when_semantic_disabled() {
        let context = ToolSchemaContext::default();
        let schema = get_schema("grep", &context).expect("grep schema");
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
    fn test_grep_schema_exposes_semantic_modes_when_enabled() {
        let context = ToolSchemaContext {
            semantic_retrieval_enabled: true,
            ..ToolSchemaContext::default()
        };
        let schema = get_schema("grep", &context).expect("grep schema");
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
        let expected: BTreeSet<&'static str> = TOOL_REGISTRY
            .iter()
            .filter(|t| matches!(t.manifest().domain, ToolDomain::Novel))
            .map(|t| t.name())
            .collect();

        let actual: BTreeSet<&'static str> = visible_tools_for_model().into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_context_tools_are_parallel_safe() {
        for name in ["outline", "character_sheet", "search_knowledge"] {
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
        for name in ["read", "edit", "create", "delete", "move", "ls", "grep"] {
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
        assert!(!visible.contains(&"edit"), "Planning should hide edit");
        assert!(!visible.contains(&"create"), "Planning should hide create");
        assert!(!visible.contains(&"delete"), "Planning should hide delete");
        assert!(!visible.contains(&"move"), "Planning should hide move");
        assert!(visible.contains(&"read"));
        assert!(visible.contains(&"ls"));
        assert!(visible.contains(&"grep"));
        assert!(visible.contains(&"outline"));
    }

    #[test]
    fn test_writing_mode_includes_all_tools() {
        let visible = visible_tools_for_mode(AgentMode::Writing);
        assert_eq!(visible.len(), 13);
        assert!(visible.contains(&"edit"));
        assert!(visible.contains(&"create"));
        assert!(visible.contains(&"delete"));
        assert!(visible.contains(&"move"));
    }
}
