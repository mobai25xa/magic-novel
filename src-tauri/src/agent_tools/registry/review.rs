use serde_json::json;

use crate::agent_tools::contracts::{ConfirmationPolicy, IdempotencyPolicy, RiskLevel, ToolDomain};
use crate::agent_tools::definition::{
    ToolCapability, ToolDefinition, ToolManifest, ToolSchemaContext, ToolVisibility,
};

pub(super) static REVIEW_CHECK_TOOL: ReviewCheckTool = ReviewCheckTool;

pub(super) struct ReviewCheckTool;

impl ToolDefinition for ReviewCheckTool {
    fn name(&self) -> &'static str {
        "review_check"
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "novel.review_check",
            llm_name: "review_check",
            domain: ToolDomain::Novel,
            risk_level: RiskLevel::Low,
            confirmation: ConfirmationPolicy::Never,
            idempotency: IdempotencyPolicy::Required,
            parallel_safe: true,
            timeout_ms: 30_000,
            capabilities: &[ToolCapability::Review],
            visibility: ToolVisibility::everywhere(),
        }
    }

    fn description(&self) -> &'static str {
        concat!(
            "Run a read-only review over one or more chapter targets and return a structured review report.\n\n",
            "Use this for manual review debugging, regression checks, or worker sandbox validation. ",
            "This tool must not write mission artifacts or persist credentials.\n\n",
            "PERFORMANCE TIP: it is parallel-safe, so independent review_check calls can run in parallel when they target separate debugging tasks.\n\n",
            "Input mirrors the review runner contract: scope_ref, target_refs, optional review_types, ",
            "branch_id, task_card_ref, context_pack_ref, effective_rules_fingerprint, severity_threshold."
        )
    }

    fn schema(&self, _context: &ToolSchemaContext) -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "scope_ref": {
                    "type": "string",
                    "description": "Review scope identifier, e.g. chapter:manuscripts/vol_1/ch_1.json"
                },
                "target_refs": {
                    "type": "array",
                    "minItems": 1,
                    "items": { "type": "string" }
                },
                "review_types": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": [
                            "word_count",
                            "continuity",
                            "logic",
                            "character",
                            "style",
                            "terminology",
                            "foreshadow",
                            "objective_completion"
                        ]
                    }
                },
                "branch_id": { "type": "string" },
                "task_card_ref": { "type": "string" },
                "context_pack_ref": { "type": "string" },
                "effective_rules_fingerprint": { "type": "string" },
                "severity_threshold": {
                    "type": "string",
                    "enum": ["warn", "block"]
                }
            },
            "required": ["scope_ref", "target_refs"],
            "additionalProperties": false
        }))
    }
}
