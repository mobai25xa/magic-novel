use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext, ToolVisibility,
};

pub(super) static WORKSPACE_MAP_TOOL: WorkspaceMapTool = WorkspaceMapTool;

pub(super) struct WorkspaceMapTool;

impl ToolDefinition for WorkspaceMapTool {
    fn name(&self) -> &'static str {
        "workspace_map"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.workspace_map",
            llm_name: "workspace_map",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::None,
            parallel_safe: true,
            timeout_ms: 30_000,
            capabilities: &[ToolCapability::WorkspaceRead],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Summarize workspace structure and progress (book/volume/knowledge).\n\n",
            "This tool returns refs in '<kind>:<project_relative_path>' form that can be passed into other tools.\n\n",
            "Use this to discover available volumes/chapters and knowledge partitions before calling context_read.\n\n",
            "PERFORMANCE TIP: workspace_map is parallel-safe; you can call it alongside other read-only tools."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "scope": {
                    "type": "string",
                    "enum": ["book", "volume", "knowledge"],
                    "description": "Mapping scope (default: book)."
                },
                "target_ref": {
                    "type": "string",
                    "description": "When scope=volume, the volume ref to map."
                },
                "depth": {
                    "type": "number",
                    "description": "Expansion depth (default: 2)."
                },
                "include_stats": {
                    "type": "boolean",
                    "description": "Include word count/status summaries (default: true)."
                },
                "include_children": {
                    "type": "boolean",
                    "description": "Include child nodes (default: true)."
                },
                "cursor": {
                    "type": "string",
                    "description": "Opaque pagination cursor for the next page."
                },
                "limit": {
                    "type": "number",
                    "description": "Max nodes per page (default: 200)."
                },
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
