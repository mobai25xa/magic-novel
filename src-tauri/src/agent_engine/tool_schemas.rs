//! OpenAI-compatible tool schema builder for the agent engine.

use crate::agent_tools::definition::ToolSchemaContext;
use crate::agent_tools::registry::{
    build_filtered_openai_tool_schema_report, build_openai_tool_schema_report,
    ToolSchemaSkipDiagnostic,
};

use super::types::{AgentMode, ClarificationMode};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BuiltToolSchemas {
    pub schemas: serde_json::Value,
    pub exposed_tools: Vec<String>,
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
    let available_workers: Vec<String> = crate::services::global_config::load_worker_definitions()
        .iter()
        .map(|w| w.name.clone())
        .collect();

    ToolSchemaContext {
        semantic_retrieval_enabled,
        clarification_mode,
        available_skills,
        available_workers,
    }
}

/// Build filtered tool schemas based on a whitelist.
pub(crate) fn build_filtered_tool_schema_bundle(
    whitelist: &[String],
    clarification_mode: ClarificationMode,
    semantic_retrieval_enabled: bool,
    mode: AgentMode,
) -> BuiltToolSchemas {
    let context = build_schema_context(clarification_mode, semantic_retrieval_enabled);
    let filtered = build_filtered_openai_tool_schema_report(whitelist, mode, &context);

    BuiltToolSchemas {
        schemas: serde_json::Value::Array(filtered.tools),
        exposed_tools: filtered.exposed_tools,
        skipped_tools: filtered.skipped_tools,
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
        skipped_tools: report.skipped_tools,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_filtered_tool_schemas(
    whitelist: &[String],
    clarification_mode: ClarificationMode,
    semantic_retrieval_enabled: bool,
    mode: AgentMode,
) -> serde_json::Value {
    build_filtered_tool_schema_bundle(
        whitelist,
        clarification_mode,
        semantic_retrieval_enabled,
        mode,
    )
    .schemas
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

    #[test]
    fn interactive_mode_exposes_askuser_even_without_write_tools() {
        let tools = tool_names(build_tool_schemas(
            ClarificationMode::Interactive,
            false,
            AgentMode::Planning,
        ));
        assert!(tools.contains(&"askuser".to_string()));
        assert!(!tools.contains(&"edit".to_string()));
        assert!(!tools.contains(&"create".to_string()));
        assert!(!tools.contains(&"delete".to_string()));
        assert!(!tools.contains(&"move".to_string()));
    }

    #[test]
    fn headless_defer_hides_askuser() {
        let tools = tool_names(build_tool_schemas(
            ClarificationMode::HeadlessDefer,
            false,
            AgentMode::Writing,
        ));
        assert!(!tools.contains(&"askuser".to_string()));
        assert!(tools.contains(&"edit".to_string()));
    }

    #[test]
    fn filtered_schemas_preserve_mode_constraints() {
        let whitelist = vec![
            "read".to_string(),
            "edit".to_string(),
            "askuser".to_string(),
        ];
        let tools = tool_names(build_filtered_tool_schemas(
            &whitelist,
            ClarificationMode::Interactive,
            false,
            AgentMode::Planning,
        ));
        assert!(tools.contains(&"read".to_string()));
        assert!(tools.contains(&"askuser".to_string()));
        assert!(!tools.contains(&"edit".to_string()));
    }

    #[test]
    fn writing_mode_exposes_delete_and_move() {
        let tools = tool_names(build_tool_schemas(
            ClarificationMode::Interactive,
            false,
            AgentMode::Writing,
        ));
        assert!(tools.contains(&"delete".to_string()));
        assert!(tools.contains(&"move".to_string()));
    }

    #[test]
    fn tool_schema_bundle_includes_exposure_diagnostics() {
        let bundle =
            build_tool_schema_bundle(ClarificationMode::Interactive, false, AgentMode::Writing);

        assert_eq!(bundle.exposed_tools.len(), 13);
        assert!(bundle.skipped_tools.is_empty());
        assert!(tool_names(bundle.schemas).contains(&"edit".to_string()));
    }
}
