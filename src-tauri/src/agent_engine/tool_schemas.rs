//! OpenAI-compatible tool schema builder for the agent engine.

use crate::agent_engine::exposure_policy::ExposureContext;
use crate::agent_tools::definition::ToolSchemaContext;
use crate::agent_tools::registry::{
    build_openai_tool_schema_report, build_openai_tool_schema_report_for_exposure,
    ToolHiddenDiagnostic, ToolSchemaSkipDiagnostic,
};

use super::types::{AgentMode, ClarificationMode};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BuiltToolSchemas {
    pub schemas: serde_json::Value,
    pub exposed_tools: Vec<String>,
    pub hidden_tools: Vec<ToolHiddenDiagnostic>,
    pub skipped_tools: Vec<ToolSchemaSkipDiagnostic>,
}

fn build_schema_context(
    clarification_mode: ClarificationMode,
    semantic_retrieval_enabled: bool,
) -> ToolSchemaContext {
    let available_skills: Vec<String> = super::skills::get_skill_definitions()
        .iter()
        .filter(|s| s.enabled)
        .map(|s| s.name.to_string())
        .collect();

    ToolSchemaContext {
        semantic_retrieval_enabled,
        clarification_mode,
        available_skills,
    }
}

fn build_schema_context_from_exposure(exposure: &ExposureContext) -> ToolSchemaContext {
    build_schema_context(
        exposure.clarification_mode,
        exposure.semantic_retrieval_enabled,
    )
}

pub(crate) fn build_tool_schema_bundle_for_exposure(
    exposure: &ExposureContext,
) -> BuiltToolSchemas {
    let context = build_schema_context_from_exposure(exposure);
    let report = build_openai_tool_schema_report_for_exposure(exposure, &context);

    BuiltToolSchemas {
        schemas: serde_json::Value::Array(report.tools),
        exposed_tools: report.exposed_tools,
        hidden_tools: report.hidden_tools,
        skipped_tools: report.skipped_tools,
    }
}

/// Build OpenAI-compatible tool schemas (aligned to TS agent prompt contract).
pub(crate) fn build_tool_schema_bundle(
    clarification_mode: ClarificationMode,
    semantic_retrieval_enabled: bool,
    mode: AgentMode,
) -> BuiltToolSchemas {
    let context = build_schema_context(clarification_mode, semantic_retrieval_enabled);
    let report = build_openai_tool_schema_report(mode, &context);

    BuiltToolSchemas {
        schemas: serde_json::Value::Array(report.tools),
        exposed_tools: report.exposed_tools,
        hidden_tools: report.hidden_tools,
        skipped_tools: report.skipped_tools,
    }
}

/// Build OpenAI-compatible tool schemas (aligned to TS agent prompt contract).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_tool_schemas(
    clarification_mode: ClarificationMode,
    semantic_retrieval_enabled: bool,
    mode: AgentMode,
) -> serde_json::Value {
    build_tool_schema_bundle(clarification_mode, semantic_retrieval_enabled, mode).schemas
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_names(value: serde_json::Value) -> Vec<String> {
        value
            .as_array()
            .expect("tool schemas should be an array")
            .iter()
            .filter_map(|tool| {
                tool.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .map(ToString::to_string)
            })
            .collect()
    }

    fn tool_parameters(value: serde_json::Value, tool_name: &str) -> serde_json::Value {
        value
            .as_array()
            .expect("tool schemas should be an array")
            .iter()
            .find_map(|tool| {
                let function = tool.get("function")?;
                let name = function.get("name")?.as_str()?;
                if name == tool_name {
                    function.get("parameters").cloned()
                } else {
                    None
                }
            })
            .expect("tool parameters should exist")
    }

    #[test]
    fn interactive_mode_exposes_askuser_even_without_write_tools() {
        let tools = tool_names(build_tool_schemas(
            ClarificationMode::Interactive,
            false,
            AgentMode::Planning,
        ));
        assert!(tools.contains(&"askuser".to_string()));
        assert!(!tools.contains(&"draft_write".to_string()));
        assert!(!tools.contains(&"structure_edit".to_string()));
        assert!(!tools.contains(&"knowledge_write".to_string()));
    }

    #[test]
    fn headless_defer_hides_askuser() {
        let tools = tool_names(build_tool_schemas(
            ClarificationMode::HeadlessDefer,
            false,
            AgentMode::Writing,
        ));
        assert!(!tools.contains(&"askuser".to_string()));
        assert!(tools.contains(&"draft_write".to_string()));
    }

    #[test]
    fn writing_mode_exposes_structure_edit() {
        let tools = tool_names(build_tool_schemas(
            ClarificationMode::Interactive,
            false,
            AgentMode::Writing,
        ));
        assert!(tools.contains(&"structure_edit".to_string()));
    }

    #[test]
    fn tool_schema_bundle_includes_exposure_diagnostics() {
        let bundle =
            build_tool_schema_bundle(ClarificationMode::Interactive, false, AgentMode::Writing);

        assert_eq!(bundle.exposed_tools.len(), 10);
        assert!(!bundle.hidden_tools.is_empty());
        assert!(bundle.skipped_tools.is_empty());
        assert!(tool_names(bundle.schemas).contains(&"draft_write".to_string()));
        assert!(bundle
            .hidden_tools
            .iter()
            .any(|tool| tool.tool_name == "skill"));
    }

    #[test]
    fn todowrite_live_schema_hides_legacy_worker_field() {
        let parameters = tool_parameters(
            build_tool_schemas(ClarificationMode::Interactive, false, AgentMode::Writing),
            "todowrite",
        );
        let item_properties = parameters
            .get("properties")
            .and_then(|value| value.get("todos"))
            .and_then(|value| value.get("items"))
            .and_then(|value| value.get("properties"))
            .and_then(|value| value.as_object())
            .expect("todowrite item properties");

        assert!(item_properties.contains_key("status"));
        assert!(item_properties.contains_key("text"));
        assert!(!item_properties.contains_key("worker"));
    }
}
