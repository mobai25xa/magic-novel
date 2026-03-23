use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext, ToolVisibility,
};

pub(super) static INSPIRATION_CONSENSUS_PATCH_TOOL: InspirationConsensusPatchTool =
    InspirationConsensusPatchTool;
pub(super) static INSPIRATION_OPEN_QUESTIONS_PATCH_TOOL: InspirationOpenQuestionsPatchTool =
    InspirationOpenQuestionsPatchTool;

pub(super) struct InspirationConsensusPatchTool;

impl ToolDefinition for InspirationConsensusPatchTool {
    fn name(&self) -> &'static str {
        "inspiration_consensus_patch"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.inspiration_consensus_patch",
            llm_name: "inspiration_consensus_patch",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: false,
            timeout_ms: 10_000,
            capabilities: &[ToolCapability::InspirationPatch],
            visibility: ToolVisibility::main_session_only(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Update inspiration-page consensus draft state for a single field.\n\n",
            "Use this tool to write candidate consensus into the left-side inspiration panel. ",
            "You may only update draft values. You must not treat draft as confirmed canon. ",
            "Do not use this tool for project files, knowledge files, or any workspace content.\n\n",
            "Locked fields will reject overwrites. Use set_text for string fields, set_items / append_items for list fields, and clear_draft to clear the current draft."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "state": consensus_state_schema(),
                "field_id": {
                    "type": "string",
                    "enum": [
                        "story_core",
                        "premise",
                        "genre_tone",
                        "protagonist",
                        "worldview",
                        "core_conflict",
                        "selling_points",
                        "audience",
                        "ending_direction"
                    ]
                },
                "operation": {
                    "type": "string",
                    "enum": ["set_text", "set_items", "append_items", "clear_draft"]
                },
                "text_value": {
                    "type": "string",
                    "description": "Required for set_text"
                },
                "items": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Required for set_items and append_items"
                },
                "source_turn_id": {
                    "type": "integer",
                    "minimum": 0
                }
            },
            "required": ["field_id", "operation"],
            "additionalProperties": false
        }))
    }
}

pub(super) struct InspirationOpenQuestionsPatchTool;

impl ToolDefinition for InspirationOpenQuestionsPatchTool {
    fn name(&self) -> &'static str {
        "inspiration_open_questions_patch"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.inspiration_open_questions_patch",
            llm_name: "inspiration_open_questions_patch",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: false,
            timeout_ms: 10_000,
            capabilities: &[ToolCapability::InspirationPatch],
            visibility: ToolVisibility::main_session_only(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Maintain the inspiration-page open-questions list.\n\n",
            "Use this tool to add a missing clarification question, or mark one as resolved or dismissed. ",
            "Do not use it for project tasks, files, or askuser-style questionnaires."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "items": open_question_schema()
                },
                "operation": {
                    "type": "string",
                    "enum": ["add", "resolve", "dismiss"]
                },
                "question_id": {
                    "type": "string",
                    "description": "Required for resolve and dismiss"
                },
                "question": {
                    "type": "string",
                    "description": "Required for add"
                },
                "importance": {
                    "type": "string",
                    "enum": ["high", "medium", "low"]
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        }))
    }
}

fn consensus_state_schema() -> serde_json::Value {
    let field = consensus_field_schema();
    json!({
        "type": "object",
        "properties": {
            "story_core": field.clone(),
            "premise": field.clone(),
            "genre_tone": field.clone(),
            "protagonist": field.clone(),
            "worldview": field.clone(),
            "core_conflict": field.clone(),
            "selling_points": field.clone(),
            "audience": field.clone(),
            "ending_direction": field
        },
        "required": [],
        "additionalProperties": false
    })
}

fn consensus_field_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "field_id": {
                "type": "string",
                "enum": [
                    "story_core",
                    "premise",
                    "genre_tone",
                    "protagonist",
                    "worldview",
                    "core_conflict",
                    "selling_points",
                    "audience",
                    "ending_direction"
                ]
            },
            "draft_value": consensus_value_schema(),
            "confirmed_value": consensus_value_schema(),
            "locked": { "type": "boolean" },
            "updated_at": { "type": "integer" },
            "last_source_turn_id": { "type": "integer", "minimum": 0 }
        },
        "required": ["field_id"],
        "additionalProperties": false
    })
}

fn consensus_value_schema() -> serde_json::Value {
    json!({
        "type": ["string", "array", "null"],
        "items": { "type": "string" }
    })
}

fn open_question_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "question_id": { "type": "string" },
            "question": { "type": "string" },
            "importance": { "type": "string", "enum": ["high", "medium", "low"] },
            "status": { "type": "string", "enum": ["open", "resolved", "dismissed"] }
        },
        "required": ["question_id", "question", "importance", "status"],
        "additionalProperties": false
    })
}
