use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext, ToolVisibility,
};

pub(super) static KNOWLEDGE_READ_TOOL: KnowledgeReadTool = KnowledgeReadTool;
pub(super) static KNOWLEDGE_WRITE_TOOL: KnowledgeWriteTool = KnowledgeWriteTool;

pub(super) struct KnowledgeReadTool;

impl ToolDefinition for KnowledgeReadTool {
    fn name(&self) -> &'static str {
        "knowledge_read"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.knowledge_read",
            llm_name: "knowledge_read",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::None,
            parallel_safe: true,
            timeout_ms: 30_000,
            capabilities: &[ToolCapability::KnowledgeRead],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Read knowledge items by ref or query and return compact card summaries.\n\n",
            "Provide item_ref (knowledge:<path>) or query. If both are omitted, set knowledge_type to list top_k items in that partition.\n\n",
            "Default view_mode=compact. Use view_mode=full only when necessary and keep outputs budgeted.\n\n",
            "PERFORMANCE TIP: knowledge_read is parallel-safe; you can fetch multiple items in parallel."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "knowledge_type": {
                    "type": "string",
                    "enum": [
                        "character",
                        "location",
                        "organization",
                        "rule",
                        "term",
                        "plotline",
                        "style_rule",
                        "source",
                        "chapter_summary",
                        "recent_fact",
                        "foreshadow"
                    ]
                },
                "item_ref": { "type": "string" },
                "query": { "type": "string" },
                "view_mode": {
                    "type": "string",
                    "enum": ["compact", "full"]
                },
                "top_k": { "type": "number" },
                "budget_chars": { "type": "number" },
                "timeout_ms": {
                    "type": "number",
                    "description": "Requested time budget (ms). Clamped to tool hard cap."
                }
            },
            "required": [],
            "additionalProperties": false
        }))
    }
}

pub(super) struct KnowledgeWriteTool;

impl ToolDefinition for KnowledgeWriteTool {
    fn name(&self) -> &'static str {
        "knowledge_write"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.knowledge_write",
            llm_name: "knowledge_write",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Medium,
            confirmation: ConfirmationPolicy::SensitiveWrite,
            idempotency: IdempotencyPolicy::Required,
            parallel_safe: false,
            timeout_ms: 60_000,
            capabilities: &[ToolCapability::KnowledgeWrite],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Propose knowledge deltas (auditable and gated).\n\n",
            "v0 supports op=propose only to generate a KnowledgeDelta rather than directly overwriting files.\n",
            "Each change.target_ref must be a knowledge ref (knowledge:.magic_novel/...).\n",
            "Use dry_run=true to preview without persisting the delta artifact."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["propose"]
                },
                "changes": {
                    "type": "array",
                    "minItems": 1,
                    "items": {
                        "type": "object",
                        "properties": {
                            "target_ref": { "type": "string" },
                            "kind": { "type": "string", "enum": ["add", "update", "delete"] },
                            "fields": { "type": "object", "additionalProperties": true }
                        },
                        "required": ["target_ref", "kind", "fields"],
                        "additionalProperties": false
                    }
                },
                "evidence_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "dry_run": { "type": "boolean" },
                "idempotency_key": { "type": "string" },
                "timeout_ms": {
                    "type": "number",
                    "description": "Requested time budget (ms). Clamped to tool hard cap."
                }
            },
            "required": ["op", "changes"],
            "additionalProperties": false
        }))
    }
}
