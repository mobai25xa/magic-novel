use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext, ToolVisibility,
};

pub(super) static CONTEXT_READ_TOOL: ContextReadTool = ContextReadTool;
pub(super) static CONTEXT_SEARCH_TOOL: ContextSearchTool = ContextSearchTool;

pub(super) struct ContextReadTool;

impl ToolDefinition for ContextReadTool {
    fn name(&self) -> &'static str {
        "context_read"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.context_read",
            llm_name: "context_read",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::None,
            parallel_safe: true,
            timeout_ms: 30_000,
            capabilities: &[ToolCapability::ContextRead],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Read a referenced target (chapter/volume/knowledge) in a compact, budgeted form.\n\n",
            "Refs use '<kind>:<project_relative_path>' (e.g. chapter:manuscripts/vol_1/ch_01.json).\n\n",
            "Use view_mode=compact by default, and increase budget_chars only when necessary.\n\n",
            "PERFORMANCE TIP: context_read is parallel-safe; you can read multiple refs in parallel."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "target_ref": {
                    "type": "string",
                    "description": "Target ref to read (chapter/volume/knowledge)."
                },
                "view_mode": {
                    "type": "string",
                    "enum": ["compact", "full"],
                    "description": "compact returns a short view; full returns more but still budgeted."
                },
                "budget_chars": {
                    "type": "number",
                    "description": "Max characters to return (default: 2000)."
                },
                "span": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["head"] },
                        "chars": { "type": "number" }
                    },
                    "required": ["kind"],
                    "additionalProperties": false,
                    "description": "Optional span selector. v0 supports head(chars)."
                }
            },
            "required": ["target_ref"],
            "additionalProperties": false
        }))
    }
}

pub(super) struct ContextSearchTool;

impl ToolDefinition for ContextSearchTool {
    fn name(&self) -> &'static str {
        "context_search"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.context_search",
            llm_name: "context_search",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::None,
            parallel_safe: true,
            timeout_ms: 60_000,
            capabilities: &[ToolCapability::Search],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Search across draft text and/or knowledge base and return evidence snippets.\n\n",
            "Use corpus=all by default, and restrict via scope.paths when possible.\n\n",
            "PERFORMANCE TIP: context_search is parallel-safe; you can run multiple searches in parallel."
        )
    }

    fn schema(&self, context: &ToolSchemaContext) -> Option<serde_json::Value> {
        let mode_enum = if context.semantic_retrieval_enabled {
            json!(["keyword", "semantic", "hybrid"])
        } else {
            json!(["keyword"])
        };

        Some(json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "corpus": {
                    "type": "string",
                    "enum": ["draft", "knowledge", "all"],
                    "description": "Corpus to search (default: all)."
                },
                "mode": {
                    "type": "string",
                    "enum": mode_enum,
                    "description": "Search mode (default: keyword)."
                },
                "top_k": {
                    "type": "number",
                    "description": "Max hits to return (default: 10)."
                },
                "scope": {
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": [],
                    "additionalProperties": false,
                    "description": "Optional path allowlist filter."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }))
    }
}
