use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolDefinition, ToolManifest, ToolSchemaContext, DEFAULT_TOOL_TIMEOUT_MS,
};

pub(super) static OUTLINE_TOOL: OutlineTool = OutlineTool;
pub(super) static CHARACTER_SHEET_TOOL: CharacterSheetTool = CharacterSheetTool;
pub(super) static SEARCH_KNOWLEDGE_TOOL: SearchKnowledgeTool = SearchKnowledgeTool;

pub(super) struct OutlineTool;
impl ToolDefinition for OutlineTool {
    fn name(&self) -> &'static str {
        "outline"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.outline",
            llm_name: "outline",
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
            "Retrieve the chapter outline for the entire book or a specific volume. ",
            "Returns titles, summaries, and word counts.\n\n",
            "PERFORMANCE TIP: outline is parallel-safe. Call it alongside read, grep, ",
            "or character_sheet in a single response to gather context faster.\n\n",
            "Use outline to understand story structure before making edits. ",
            "Prefer read for the actual chapter content."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "volume_path": {
                    "type": "string",
                    "description": "Volume path (optional; omit to get the full book outline)"
                },
                "include_summary": {
                    "type": "boolean",
                    "description": "Whether to include chapter summaries (default true)"
                }
            },
            "required": [],
            "additionalProperties": false
        }))
    }
}

pub(super) struct CharacterSheetTool;
impl ToolDefinition for CharacterSheetTool {
    fn name(&self) -> &'static str {
        "character_sheet"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.character_sheet",
            llm_name: "character_sheet",
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
            "Read a character profile card. Omit 'name' to list all character names.\n\n",
            "PERFORMANCE TIP: character_sheet is parallel-safe. Call it alongside outline, ",
            "read, or grep in a single response when gathering context about characters.\n\n",
            "Use character_sheet to check consistency before writing scenes involving a character. ",
            "Prefer grep to find where a character appears across chapters."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Character name (fuzzy match)"
                }
            },
            "required": [],
            "additionalProperties": false
        }))
    }
}

pub(super) struct SearchKnowledgeTool;
impl ToolDefinition for SearchKnowledgeTool {
    fn name(&self) -> &'static str {
        "search_knowledge"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.search_knowledge",
            llm_name: "search_knowledge",
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
            "Search the project knowledge base (.magic_novel/) for keywords. ",
            "Returns matching file snippets from world-building docs, style guides, and notes.\n\n",
            "PERFORMANCE TIP: search_knowledge is parallel-safe. Call it alongside outline, ",
            "character_sheet, or grep in a single response.\n\n",
            "Prefer grep for searching chapter content. ",
            "Use search_knowledge specifically for knowledge base files (world settings, rules, notes)."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search keywords"
                },
                "top_k": {
                    "type": "number",
                    "description": "Number of results to return (default 5)"
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }))
    }
}
