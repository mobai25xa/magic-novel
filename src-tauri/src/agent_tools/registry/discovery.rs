use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolDefinition, ToolManifest, ToolSchemaContext, DEFAULT_TOOL_TIMEOUT_MS,
};

pub(super) static LS_TOOL: LsTool = LsTool;
pub(super) static GREP_TOOL: GrepTool = GrepTool;

pub(super) struct LsTool;
impl ToolDefinition for LsTool {
    fn name(&self) -> &'static str {
        "ls"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.ls",
            llm_name: "ls",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: true,
            timeout_ms: DEFAULT_TOOL_TIMEOUT_MS,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "List project structure: volumes, chapters, and knowledge base folders/files.\n\n",
            "Use '.' for root, a volume path for its chapters, or '.magic_novel' for the knowledge base.\n\n",
            "Prefer grep or search_knowledge for content search — ls only shows structure, not content.\n\n",
            "PERFORMANCE TIP: When exploring the project, make parallel ls calls for different paths ",
            "in a single response. ls is parallel-safe.\n\n",
            "Results are paginated. If truncated, use offset/limit to fetch more."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Optional path. Use '.' for root, a volume path, or '.magic_novel' for knowledge base"
                },
                "offset": { "type": "number", "description": "Skip first N items (default 0)" },
                "limit": { "type": "number", "description": "Max items to return (default 30)" }
            },
            "required": [],
            "additionalProperties": false
        }))
    }
}

pub(super) struct GrepTool;
impl ToolDefinition for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.grep",
            llm_name: "grep",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: true,
            timeout_ms: 60_000,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "Search project corpus for evidence snippets.\n\n",
            "Use grep to find specific text, character names, plot elements, or phrases across chapters.\n\n",
            "PERFORMANCE TIP: When searching for multiple patterns or exploring the corpus, make parallel ",
            "grep calls with different queries in a single response. grep is parallel-safe.\n\n",
            "Prefer search_knowledge for knowledge base files (.magic_novel/). ",
            "Prefer read for reading a specific chapter you already know the path of.\n\n",
            "DO NOT use grep when you already have the chapter path — use read instead."
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
                "mode": {
                    "type": "string",
                    "enum": mode_enum
                },
                "top_k": { "type": "number" },
                "scope": {
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": [],
                    "additionalProperties": false
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }))
    }
}
