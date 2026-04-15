use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::messages::AgentMessage;

pub(crate) const MAX_RECOVERY_ATTEMPTS: u8 = 2;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ToolExecutionDiagnostic {
    pub tool_name: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retryable: bool,
    pub details: Option<Value>,
    pub args: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct RecoveryDirective {
    pub system_message: Option<AgentMessage>,
    pub budget_exhausted: bool,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct RecoveryBudget {
    attempts: HashMap<String, u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryIssue {
    key: String,
    guidance: String,
    exhausted_guidance: String,
}

impl RecoveryBudget {
    pub(crate) fn observe(&mut self, diagnostics: &[ToolExecutionDiagnostic]) -> RecoveryDirective {
        let mut seen = HashSet::new();
        let mut recover_lines = Vec::new();
        let mut exhausted_lines = Vec::new();

        for issue in diagnostics
            .iter()
            .filter_map(classify_recovery_issue)
            .filter(|issue| seen.insert(issue.key.clone()))
        {
            let attempt = self.bump(&issue.key);
            if attempt > MAX_RECOVERY_ATTEMPTS {
                exhausted_lines.push(issue.exhausted_guidance);
            } else {
                recover_lines.push(format!(
                    "- {} (recovery {}/{})",
                    issue.guidance, attempt, MAX_RECOVERY_ATTEMPTS
                ));
            }
        }

        if recover_lines.is_empty() && exhausted_lines.is_empty() {
            return RecoveryDirective {
                system_message: None,
                budget_exhausted: false,
            };
        }

        let mut lines = Vec::new();
        if !recover_lines.is_empty() {
            lines.push(
                "System recovery note: continue the task and treat recoverable tool errors as internal execution state, not as user-facing terminal failures."
                    .to_string(),
            );
            lines.extend(recover_lines);
        }
        if !exhausted_lines.is_empty() {
            lines.push(
                "System recovery note: recovery budget is exhausted for repeated tool failures. Do not issue more tool calls for the same blocked target in this turn; explain the blocker to the user briefly."
                    .to_string(),
            );
            lines.extend(exhausted_lines.into_iter().map(|line| format!("- {line}")));
        }

        RecoveryDirective {
            system_message: Some(AgentMessage::system(lines.join("\n"))),
            budget_exhausted: !lines.is_empty()
                && diagnostics
                    .iter()
                    .filter_map(classify_recovery_issue)
                    .any(|issue| {
                        self.attempts.get(&issue.key).copied().unwrap_or_default()
                            > MAX_RECOVERY_ATTEMPTS
                    }),
        }
    }

    fn bump(&mut self, key: &str) -> u8 {
        let entry = self.attempts.entry(key.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        *entry
    }
}

fn classify_recovery_issue(diagnostic: &ToolExecutionDiagnostic) -> Option<RecoveryIssue> {
    let error_code = diagnostic.error_code.as_deref()?;
    match error_code {
        "E_TOOL_SCHEMA_INVALID" if diagnostic.tool_name == "knowledge_write" => {
            let field_path = knowledge_write_fields_issue_path(diagnostic)?;
            Some(RecoveryIssue {
                key: format!("knowledge_write_fields:{field_path}"),
                guidance: format!(
                    "Fix `{field_path}` before retrying `knowledge_write`. It must be a JSON object such as {{\"summary\":\"canon update\"}}, not a string, array, or patch list"
                ),
                exhausted_guidance: format!(
                    "`knowledge_write` is still failing on `{field_path}` after recovery attempts; stop retrying the same malformed payload"
                ),
            })
        }
        "E_TOOL_SCHEMA_INVALID" if diagnostic.tool_name == "askuser" => Some(RecoveryIssue {
            key: format!("askuser_schema:{}", askuser_issue_key(diagnostic)),
            guidance: "Fix `askuser` before retrying. Send an object with either `questionnaire` as a non-empty string, or `questions` as 1-4 items where each item includes non-empty `question`, `topic`, and `options` with 2-4 non-empty strings".to_string(),
            exhausted_guidance: "`askuser` is still failing schema validation after recovery attempts; stop retrying the same malformed questionnaire payload".to_string(),
        }),
        "E_TOOL_SCHEMA_INVALID" if diagnostic.tool_name == "structure_edit" => {
            if structure_edit_node_type_issue(diagnostic) {
                return Some(RecoveryIssue {
                    key: "structure_edit:node_type".to_string(),
                    guidance: "Fix `structure_edit.node_type` before retrying. Only `volume` and `chapter` are supported; `knowledge_item` is not available".to_string(),
                    exhausted_guidance: "`structure_edit` is still failing on `node_type`; stop retrying the same unsupported node type".to_string(),
                });
            }

            Some(RecoveryIssue {
                key: "structure_edit:schema".to_string(),
                guidance: "Re-check `structure_edit` inputs before retrying. Use `op` + `node_type`, and include the required refs/title/position fields for that operation".to_string(),
                exhausted_guidance: "`structure_edit` is still failing schema validation after recovery attempts; stop retrying the same malformed payload".to_string(),
            })
        }
        "E_TOOL_NOT_ALLOWED" => Some(RecoveryIssue {
            key: format!("tool_not_allowed:{}", diagnostic.tool_name),
            guidance: format!(
                "Do not surface tool-availability errors. Continue with the stable core toolset and retry the task flow with the appropriate tool for `{}`",
                diagnostic.tool_name
            ),
            exhausted_guidance: format!(
                "Tool availability for `{}` has failed repeatedly; stop retrying hidden tools and respond with the current blocker",
                diagnostic.tool_name
            ),
        }),
        "E_REF_NOT_FOUND" if is_structure_target(diagnostic) => {
            let target = recovery_target(diagnostic);
            Some(RecoveryIssue {
                key: format!("missing_structure:{target}"),
                guidance: format!(
                    "The target structure `{target}` is missing. Use `workspace_map` to inspect refs, then `structure_edit` to create or repair the chapter/volume before retrying"
                ),
                exhausted_guidance: format!(
                    "The structure target `{target}` is still missing after recovery attempts; stop retrying the same missing ref"
                ),
            })
        }
        "E_REF_INVALID" if is_knowledge_ref_issue(diagnostic) => {
            let target = recovery_target(diagnostic);
            Some(RecoveryIssue {
                key: format!("invalid_knowledge_ref:{target}"),
                guidance: format!(
                    "Canonicalize the knowledge ref `{target}` before retrying. Prefer refs under `knowledge:.magic_novel/...`"
                ),
                exhausted_guidance: format!(
                    "The knowledge ref `{target}` remains invalid after recovery attempts; stop retrying the same invalid ref"
                ),
            })
        }
        "E_REF_INVALID" => {
            let target = recovery_target(diagnostic);
            Some(RecoveryIssue {
                key: format!("invalid_ref:{}:{target}", diagnostic.tool_name),
                guidance: format!(
                    "The ref `{target}` is invalid. Inspect current refs before retrying `{}`",
                    diagnostic.tool_name
                ),
                exhausted_guidance: format!(
                    "The ref `{target}` is still invalid for `{}`; stop retrying the same invalid target",
                    diagnostic.tool_name
                ),
            })
        }
        _ => None,
    }
}

fn is_structure_target(diagnostic: &ToolExecutionDiagnostic) -> bool {
    if matches!(
        diagnostic.tool_name.as_str(),
        "draft_write" | "structure_edit"
    ) {
        return true;
    }

    let target = recovery_target(diagnostic);
    target.starts_with("chapter:") || target.starts_with("volume:")
}

fn is_knowledge_ref_issue(diagnostic: &ToolExecutionDiagnostic) -> bool {
    if diagnostic.tool_name == "knowledge_read" || diagnostic.tool_name == "knowledge_write" {
        return true;
    }

    let target = recovery_target(diagnostic).to_ascii_lowercase();
    target.starts_with("knowledge:")
        || target.contains(".magic_novel/")
        || target.contains("characters/")
        || target.contains("settings/")
        || target.contains("terms/")
}

fn recovery_target(diagnostic: &ToolExecutionDiagnostic) -> String {
    for key in ["target_ref", "parent_ref", "ref", "path"] {
        if let Some(value) = diagnostic.args.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if diagnostic.tool_name == "knowledge_write" {
        if let Some(value) = diagnostic
            .args
            .get("changes")
            .and_then(Value::as_array)
            .and_then(|changes| changes.first())
            .and_then(|change| change.get("target_ref"))
            .and_then(Value::as_str)
        {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    diagnostic
        .details
        .as_ref()
        .and_then(|details| details.get("target_ref").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| diagnostic.tool_name.clone())
}

fn knowledge_write_fields_issue_path(diagnostic: &ToolExecutionDiagnostic) -> Option<String> {
    let changes = diagnostic.args.get("changes")?.as_array()?;

    for (idx, change) in changes.iter().enumerate() {
        let change_obj = change.as_object()?;
        match change_obj.get("fields") {
            Some(fields) if fields.is_object() => {}
            Some(_) | None => return Some(format!("changes[{idx}].fields")),
        }
    }

    None
}

fn askuser_issue_key(diagnostic: &ToolExecutionDiagnostic) -> &'static str {
    let message = diagnostic
        .error_message
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if message.contains("questionnaire") {
        "questionnaire"
    } else {
        "questions"
    }
}

fn structure_edit_node_type_issue(diagnostic: &ToolExecutionDiagnostic) -> bool {
    if diagnostic
        .args
        .get("node_type")
        .and_then(Value::as_str)
        .map(|value| value.eq_ignore_ascii_case("knowledge_item"))
        .unwrap_or(false)
    {
        return true;
    }

    diagnostic
        .error_message
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .contains("knowledge_item")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn diagnostic(tool_name: &str, error_code: &str, args: Value) -> ToolExecutionDiagnostic {
        ToolExecutionDiagnostic {
            tool_name: tool_name.to_string(),
            error_code: Some(error_code.to_string()),
            error_message: None,
            retryable: true,
            details: None,
            args,
        }
    }

    fn diagnostic_with_message(
        tool_name: &str,
        error_code: &str,
        error_message: &str,
        args: Value,
    ) -> ToolExecutionDiagnostic {
        ToolExecutionDiagnostic {
            tool_name: tool_name.to_string(),
            error_code: Some(error_code.to_string()),
            error_message: Some(error_message.to_string()),
            retryable: true,
            details: None,
            args,
        }
    }

    #[test]
    fn missing_structure_ref_generates_recovery_hint() {
        let mut budget = RecoveryBudget::default();
        let directive = budget.observe(&[diagnostic(
            "draft_write",
            "E_REF_NOT_FOUND",
            json!({ "target_ref": "chapter:manuscripts/vol_1/ch_9.json" }),
        )]);

        let message = directive.system_message.expect("recovery message");
        let content = message.text_content();
        assert!(content.contains("structure_edit"));
        assert!(content.contains("workspace_map"));
        assert!(!directive.budget_exhausted);
    }

    #[test]
    fn repeated_issue_exhausts_budget_on_third_attempt() {
        let mut budget = RecoveryBudget::default();
        let diagnostic = diagnostic(
            "draft_write",
            "E_REF_NOT_FOUND",
            json!({ "target_ref": "chapter:manuscripts/vol_1/ch_9.json" }),
        );

        assert!(
            !budget
                .observe(std::slice::from_ref(&diagnostic))
                .budget_exhausted
        );
        assert!(
            !budget
                .observe(std::slice::from_ref(&diagnostic))
                .budget_exhausted
        );
        assert!(
            budget
                .observe(std::slice::from_ref(&diagnostic))
                .budget_exhausted
        );
    }

    #[test]
    fn invalid_knowledge_ref_mentions_canonical_root() {
        let mut budget = RecoveryBudget::default();
        let directive = budget.observe(&[diagnostic(
            "context_read",
            "E_REF_INVALID",
            json!({ "target_ref": "knowledge:characters/hero.md" }),
        )]);

        let content = directive
            .system_message
            .expect("recovery message")
            .text_content();
        assert!(content.contains("knowledge:.magic_novel"));
    }

    #[test]
    fn knowledge_write_fields_issue_mentions_object_contract() {
        let mut budget = RecoveryBudget::default();
        let directive = budget.observe(&[diagnostic(
            "knowledge_write",
            "E_TOOL_SCHEMA_INVALID",
            json!({
                "changes": [{
                    "target_ref": "knowledge:.magic_novel/terms/foo.json",
                    "kind": "add",
                    "fields": "summary = foo"
                }]
            }),
        )]);

        let content = directive
            .system_message
            .expect("recovery message")
            .text_content();
        assert!(content.contains("changes[0].fields"));
        assert!(content.contains("JSON object"));
        assert!(content.contains("patch list"));
    }

    #[test]
    fn askuser_schema_issue_mentions_question_shape() {
        let mut budget = RecoveryBudget::default();
        let directive = budget.observe(&[diagnostic_with_message(
            "askuser",
            "E_TOOL_SCHEMA_INVALID",
            "askuser questions[0].options must contain between 2 and 4 items",
            json!({
                "questions": [{
                    "question": "Pick one",
                    "topic": "style",
                    "options": ["Only one"]
                }]
            }),
        )]);

        let content = directive
            .system_message
            .expect("recovery message")
            .text_content();
        assert!(content.contains("questionnaire"));
        assert!(content.contains("questions"));
        assert!(content.contains("2-4"));
    }

    #[test]
    fn structure_edit_schema_issue_mentions_supported_node_types() {
        let mut budget = RecoveryBudget::default();
        let directive = budget.observe(&[diagnostic_with_message(
            "structure_edit",
            "E_TOOL_SCHEMA_INVALID",
            "unknown variant `knowledge_item`, expected `volume` or `chapter`",
            json!({
                "op": "create",
                "node_type": "knowledge_item",
                "title": "Lore"
            }),
        )]);

        let content = directive
            .system_message
            .expect("recovery message")
            .text_content();
        assert!(content.contains("volume"));
        assert!(content.contains("chapter"));
        assert!(content.contains("knowledge_item"));
    }
}
