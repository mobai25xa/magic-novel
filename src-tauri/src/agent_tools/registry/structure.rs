use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext, ToolVisibility,
};

pub(super) static STRUCTURE_EDIT_TOOL: StructureEditTool = StructureEditTool;

pub(super) struct StructureEditTool;

impl ToolDefinition for StructureEditTool {
    fn name(&self) -> &'static str {
        "structure_edit"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.structure_edit",
            llm_name: "structure_edit",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Medium,
            confirmation: ConfirmationPolicy::SensitiveWrite,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: false,
            timeout_ms: 60_000,
            capabilities: &[ToolCapability::StructureWrite],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Perform structural write operations: create/move/rename/archive/restore.\n\n",
            "Refs must be '<kind>:<project_relative_path>' (chapter:/volume: refs come from workspace_map).\n\n",
            "Use dry_run=true to preview impact. Use idempotency_key for safe retries."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["create", "move", "rename", "archive", "restore"]
                },
                "node_type": {
                    "type": "string",
                    "enum": ["volume", "chapter", "knowledge_item"]
                },
                "target_ref": { "type": "string" },
                "parent_ref": { "type": "string" },
                "position": {
                    "type": "number",
                    "description": "0-based insertion index for move/create."
                },
                "title": { "type": "string" },
                "dry_run": { "type": "boolean" },
                "idempotency_key": { "type": "string" },
                "timeout_ms": {
                    "type": "number",
                    "description": "Requested time budget (ms). Clamped to tool hard cap."
                }
            },
            "required": ["op", "node_type"],
            "additionalProperties": false
        }))
    }
}
