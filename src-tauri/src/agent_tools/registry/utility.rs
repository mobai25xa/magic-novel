use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{ToolDefinition, ToolManifest, ToolSchemaContext};

pub(super) static ASKUSER_TOOL: AskuserTool = AskuserTool;
pub(super) static SKILL_TOOL: SkillTool = SkillTool;
pub(super) static TODOWRITE_TOOL: TodowriteTool = TodowriteTool;

pub(super) struct AskuserTool;
impl ToolDefinition for AskuserTool {
    fn name(&self) -> &'static str {
        "askuser"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.askuser",
            llm_name: "askuser",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::None,
            parallel_safe: false,
            timeout_ms: 120_000,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "Present 1-4 multiple-choice clarification questions to the user and wait for answers.\n\n",
            "Prefer structured questions[]. questionnaire is supported as a legacy fallback string when needed.\n",
            "Each question requires question/topic/options (2-4 options).\n",
            "You can call askuser more than once if new ambiguity appears during execution."
        )
    }
    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 4,
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": { "type": "string" },
                            "topic": { "type": "string" },
                            "options": {
                                "type": "array",
                                "minItems": 2,
                                "maxItems": 4,
                                "items": { "type": "string" }
                            }
                        },
                        "required": ["question", "topic", "options"],
                        "additionalProperties": false
                    }
                },
                "questionnaire": {
                    "type": "string",
                    "description": "Legacy fallback questionnaire DSL string"
                }
            },
            "required": [],
            "additionalProperties": false
        }))
    }
    fn externally_handled(&self) -> bool {
        true
    }
}

pub(super) struct SkillTool;
impl ToolDefinition for SkillTool {
    fn name(&self) -> &'static str {
        "skill"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.skill",
            llm_name: "skill",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: false,
            timeout_ms: 10_000,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "Legacy compatibility path for activating a named skill profile.\n\n",
            "Preferred path: set active_skill via session/orchestrator state before the turn starts. ",
            "Only invoke this tool when the compatibility path is explicitly enabled.\n\n",
            "A skill stays active for the session. Do not invoke skill repeatedly."
        )
    }
    fn schema(&self, context: &ToolSchemaContext) -> Option<serde_json::Value> {
        let mut skill_field = json!({
            "type": "string",
            "description": "Skill profile to activate"
        });

        if !context.available_skills.is_empty() {
            skill_field["enum"] = json!(context.available_skills);
        }

        Some(json!({
            "type": "object",
            "properties": {
                "skill": skill_field
            },
            "required": ["skill"],
            "additionalProperties": false
        }))
    }
    fn externally_handled(&self) -> bool {
        true
    }
}

pub(super) struct TodowriteTool;
impl ToolDefinition for TodowriteTool {
    fn name(&self) -> &'static str {
        "todowrite"
    }
    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.todowrite",
            llm_name: "todowrite",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::Optional,
            parallel_safe: false,
            timeout_ms: 10_000,
        }
    }
    fn description(&self) -> &'static str {
        concat!(
            "Create or update a task checklist to track multi-step work.\n\n",
            "Canonical input only supports todos[] with {status, text}.\n",
            "Status must be one of pending/in_progress/completed.\n",
            "At most 50 items, text length <= 500, and at most one in_progress item."
        )
    }
    fn schema(&self, context: &ToolSchemaContext) -> Option<serde_json::Value> {
        let mut item_properties = json!({
            "status": {
                "type": "string",
                "enum": ["pending", "in_progress", "completed"]
            },
            "text": {
                "type": "string",
                "maxLength": 500
            },
            "worker": {
                "type": "string",
                "description": "Optional worker routing target"
            }
        });

        if !context.available_workers.is_empty() {
            item_properties["worker"] = json!({
                "type": "string",
                "enum": context.available_workers,
                "description": "Optional worker routing target"
            });
        }

        Some(json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "maxItems": 50,
                    "items": {
                        "type": "object",
                        "properties": item_properties,
                        "required": ["status", "text"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["todos"],
            "additionalProperties": false
        }))
    }
}
