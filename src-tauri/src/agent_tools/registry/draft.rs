use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext, ToolVisibility,
};

pub(super) static DRAFT_WRITE_TOOL: DraftWriteTool = DraftWriteTool;

pub(super) struct DraftWriteTool;

impl ToolDefinition for DraftWriteTool {
    fn name(&self) -> &'static str {
        "draft_write"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.draft_write",
            llm_name: "draft_write",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Medium,
            confirmation: ConfirmationPolicy::SensitiveWrite,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: false,
            timeout_ms: 60_000,
            capabilities: &[ToolCapability::DraftWrite],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Write or revise draft text for a target chapter ref.\n\n",
            "target_ref must be a chapter ref in '<kind>:<project_relative_path>' form (e.g. chapter:manuscripts/vol_1/ch_01.json).\n\n",
            "Use dry_run=true for preview. Output should be compact (diff_summary/snippet), not full chapter content.\n\n",
            "IMPORTANT: Use only schema fields; do not invent extra parameters."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "target_ref": { "type": "string" },
                "write_mode": {
                    "type": "string",
                    "enum": ["draft", "continue", "rewrite", "polish", "retone", "compress", "expand"]
                },
                "instruction": { "type": "string" },
                "content": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["markdown"] },
                        "value": { "type": "string" }
                    },
                    "required": ["kind", "value"],
                    "additionalProperties": false
                },
                "constraints": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "context_refs": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "length_target": { "type": "number" },
                "dry_run": { "type": "boolean" },
                "idempotency_key": { "type": "string" },
                "timeout_ms": {
                    "type": "number",
                    "description": "Requested time budget (ms). Clamped to tool hard cap."
                }
            },
            "required": ["target_ref", "write_mode", "instruction", "content"],
            "additionalProperties": false
        }))
    }
}
