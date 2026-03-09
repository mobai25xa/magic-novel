use std::sync::LazyLock;

use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolDefinition, ToolManifest, ToolSchemaContext, DEFAULT_TOOL_TIMEOUT_MS,
};

pub(super) static READ_TOOL: ReadTool = ReadTool;
pub(super) static EDIT_TOOL: EditTool = EditTool;
pub(super) static CREATE_TOOL: CreateTool = CreateTool;
pub(super) static DELETE_TOOL: DeleteTool = DeleteTool;
pub(super) static MOVE_TOOL: MoveTool = MoveTool;

static EDIT_TOOL_SCHEMA: LazyLock<serde_json::Value> = LazyLock::new(|| build_edit_tool_schema());

fn build_edit_tool_schema() -> serde_json::Value {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "target".to_string(),
        json!({
            "type": "string",
            "enum": ["volume_meta", "chapter_meta", "chapter_content"],
            "description": "Edit target. volume_meta/chapter_meta edit metadata; chapter_content edits chapter blocks."
        }),
    );

    extend_edit_properties(&mut properties, edit_volume_meta_properties());
    extend_edit_properties(&mut properties, edit_chapter_meta_properties());
    extend_edit_properties(&mut properties, edit_chapter_content_properties());

    properties.insert(
        "dry_run".to_string(),
        json!({
            "type": "boolean",
            "description": "Preview only when true."
        }),
    );

    json!({
        "type": "object",
        "properties": properties,
        "required": ["target", "path"],
        "additionalProperties": false
    })
}

fn extend_edit_properties(
    target: &mut serde_json::Map<String, serde_json::Value>,
    source: serde_json::Map<String, serde_json::Value>,
) {
    for (key, value) in source {
        target.entry(key).or_insert(value);
    }
}

fn edit_volume_meta_properties() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "path".to_string(),
        json!({
            "type": "string",
            "description": "Volume path or chapter path, depending on target."
        }),
    );
    properties.insert(
        "title".to_string(),
        json!({
            "type": "string",
            "description": "Optional title for volume_meta or chapter_meta edits."
        }),
    );
    properties.insert(
        "summary".to_string(),
        json!({
            "type": "string",
            "description": "Optional summary for volume_meta or chapter_meta edits."
        }),
    );
    properties
}

fn edit_chapter_meta_properties() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = edit_volume_meta_properties();
    properties.insert(
        "status".to_string(),
        json!({
            "type": "string",
            "enum": ["draft", "revised", "final"],
            "description": "Optional chapter status for chapter_meta edits."
        }),
    );
    properties.insert(
        "target_words".to_string(),
        json!({
            "type": "integer",
            "description": "Optional target word count for chapter_meta edits."
        }),
    );
    properties.insert(
        "tags".to_string(),
        json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Optional chapter tags for chapter_meta edits."
        }),
    );
    properties.insert(
        "pinned_assets".to_string(),
        json!({
            "type": "array",
            "items": { "type": "object" },
            "description": "Optional pinned asset refs for chapter_meta edits; server validates the exact shape."
        }),
    );
    properties
}

fn edit_chapter_content_properties() -> serde_json::Map<String, serde_json::Value> {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "path".to_string(),
        json!({
            "type": "string",
            "description": "Volume path or chapter path, depending on target."
        }),
    );
    properties.insert(
        "base_revision".to_string(),
        json!({
            "type": "integer",
            "description": "Required for chapter_content edits; use the revision returned by read(kind=chapter, view=snapshot)."
        }),
    );
    properties.insert(
        "snapshot_id".to_string(),
        json!({
            "type": "string",
            "description": "Required for chapter_content edits; use the snapshot.snapshot_id returned by read(kind=chapter, view=snapshot)."
        }),
    );
    properties.insert(
        "ops".to_string(),
        json!({
            "type": "array",
            "minItems": 1,
            "description": "Required for chapter_content edits. Each op is validated server-side based on its op name.",
            "items": {
                "type": "object",
                "properties": {
                    "op": {
                        "type": "string",
                        "enum": [
                            "replace_block",
                            "delete_block",
                            "insert_before",
                            "insert_after",
                            "append_blocks",
                            "replace_range"
                        ]
                    },
                    "block_id": { "type": "string" },
                    "start_block_id": { "type": "string" },
                    "end_block_id": { "type": "string" },
                    "markdown": { "type": "string" },
                    "blocks": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "properties": {
                                "markdown": { "type": "string" }
                            },
                            "required": ["markdown"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["op"],
                "additionalProperties": false
            }
        }),
    );
    properties
}

pub(super) struct ReadTool;
impl ToolDefinition for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.read",
            llm_name: "read",
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
            "Read volume/chapter data from the project.\n\n",
            "kind=volume only supports view=meta. kind=chapter supports view=meta|snapshot|json.\n\n",
            "IMPORTANT: For chapter content edits, call read(kind=chapter, view=snapshot) first ",
            "and use both revision and snapshot.snapshot_id in edit(target=chapter_content).\n\n",
            "read is parallel-safe for multi-file exploration."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": ["volume", "chapter"], "description": "Read target kind" },
                "path": { "type": "string", "description": "Volume path (kind=volume) or chapter path (kind=chapter)" },
                "view": { "type": "string", "enum": ["meta", "snapshot", "json"], "description": "volume only supports meta; chapter supports meta/snapshot/json" }
            },
            "required": ["kind", "path", "view"],
            "additionalProperties": false
        }))
    }
}

pub(super) struct EditTool;
impl ToolDefinition for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.edit",
            llm_name: "edit",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Medium,
            confirmation: ConfirmationPolicy::SensitiveWrite,
            idempotency: IdempotencyPolicy::Required,
            parallel_safe: false,
            timeout_ms: 60_000,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "Edit volume/chapter metadata or chapter content.\n\n",
            "target=volume_meta supports: title, summary.\n",
            "target=chapter_meta supports: title, summary, status, target_words, tags, pinned_assets.\n",
            "target=chapter_content requires: base_revision, snapshot_id, ops[].\n",
            "Supported ops: replace_block, delete_block, insert_before, insert_after, append_blocks, replace_range.\n\n",
            "IMPORTANT: Always call read(kind=chapter, view=snapshot) before chapter content edits, ",
            "then use the returned revision + snapshot_id."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(EDIT_TOOL_SCHEMA.clone())
    }
}

pub(super) struct CreateTool;
impl ToolDefinition for CreateTool {
    fn name(&self) -> &'static str {
        "create"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.create",
            llm_name: "create",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Medium,
            confirmation: ConfirmationPolicy::SensitiveWrite,
            idempotency: IdempotencyPolicy::Required,
            parallel_safe: false,
            timeout_ms: 60_000,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "Create a volume or chapter.\n\n",
            "kind=volume requires title only.\n",
            "kind=chapter requires title and volume_path.\n\n",
            "If the user does not specify target volume for chapter creation, askuser first."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": ["volume", "chapter"] },
                "title": { "type": "string", "description": "Volume/chapter title" },
                "volume_path": { "type": "string", "description": "Required when kind=chapter" }
            },
            "required": ["kind", "title"],
            "additionalProperties": false
        }))
    }
}

pub(super) struct DeleteTool;
impl ToolDefinition for DeleteTool {
    fn name(&self) -> &'static str {
        "delete"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.delete",
            llm_name: "delete",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Medium,
            confirmation: ConfirmationPolicy::Always,
            idempotency: IdempotencyPolicy::Required,
            parallel_safe: false,
            timeout_ms: 60_000,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "Delete a chapter or volume by moving it to recycle bin (not permanent delete).\n\n",
            "Use dry_run=true to preview impact before commit."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": ["volume", "chapter"] },
                "path": { "type": "string", "description": "Volume path or chapter path" },
                "dry_run": { "type": "boolean", "description": "Preview only when true" }
            },
            "required": ["kind", "path"],
            "additionalProperties": false
        }))
    }
}

pub(super) struct MoveTool;
impl ToolDefinition for MoveTool {
    fn name(&self) -> &'static str {
        "move"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.move",
            llm_name: "move",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Medium,
            confirmation: ConfirmationPolicy::SensitiveWrite,
            idempotency: IdempotencyPolicy::Required,
            parallel_safe: false,
            timeout_ms: 60_000,
        }
    }
    fn description(&self) -> &'static str {
        "Move/reorder chapter into target volume and index."
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "chapter_path": { "type": "string", "description": "Source chapter path" },
                "target_volume_path": { "type": "string", "description": "Destination volume path" },
                "target_index": { "type": "integer", "description": "Insert index in target volume" },
                "dry_run": { "type": "boolean", "description": "Preview only when true" }
            },
            "required": ["chapter_path", "target_volume_path", "target_index"],
            "additionalProperties": false
        }))
    }
}
