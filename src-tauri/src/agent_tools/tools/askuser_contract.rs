use serde_json::Value;

const ASKUSER_FIELDS: &[&str] = &["questions", "questionnaire"];

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parser_contract_fields() -> &'static [&'static str] {
    ASKUSER_FIELDS
}

pub(crate) fn validate_askuser_args(args: &Value) -> Result<(), String> {
    reject_unknown_fields(args, ASKUSER_FIELDS)?;

    let has_questions = args
        .get("questions")
        .and_then(|value| value.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);
    let has_questionnaire = args
        .get("questionnaire")
        .and_then(|value| value.as_str())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    if !has_questions && !has_questionnaire {
        return Err("askuser args: requires questions or questionnaire".to_string());
    }

    Ok(())
}

pub(crate) fn extract_questionnaire(args: &Value) -> Option<String> {
    let raw = args.get("questionnaire").and_then(|v| v.as_str())?;

    let normalized = raw.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Extract structured questions from askuser args.
/// Returns a validated JSON array value for the event payload.
pub(crate) fn extract_askuser_questions(args: &Value) -> Option<serde_json::Value> {
    let questions = args.get("questions")?;
    let arr = questions.as_array()?;

    if arr.is_empty() || arr.len() > 4 {
        return None;
    }

    for q in arr {
        let question = q.get("question").and_then(|v| v.as_str()).unwrap_or("");
        let topic = q.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        let options = q.get("options").and_then(|v| v.as_array());

        if question.trim().is_empty() || topic.trim().is_empty() {
            return None;
        }

        match options {
            Some(opts) if opts.len() >= 2 && opts.len() <= 4 => {
                for opt in opts {
                    if opt.as_str().map(|s| s.trim().is_empty()).unwrap_or(true) {
                        return None;
                    }
                }
            }
            _ => return None,
        }
    }

    Some(questions.clone())
}

fn reject_unknown_fields(args: &Value, fields: &[&str]) -> Result<(), String> {
    let Some(map) = args.as_object() else {
        return Ok(());
    };

    for key in map.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(format!("askuser args: unknown field '{key}'"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;

    use super::*;

    #[test]
    fn validate_askuser_args_requires_supported_payload() {
        let err = validate_askuser_args(&json!({})).expect_err("should fail");
        assert!(err.contains("requires questions or questionnaire"));
    }

    #[test]
    fn validate_askuser_args_rejects_unknown_fields() {
        let err = validate_askuser_args(&json!({
            "questions": [{
                "question": "Continue?",
                "topic": "confirm",
                "options": ["Yes", "No"]
            }],
            "extra": true
        }))
        .expect_err("should fail");
        assert!(err.contains("unknown field"));
    }

    #[test]
    fn extract_questionnaire_trims() {
        let args = json!({"questionnaire": " 1. [question] A\n[topic] T\n[option] X\n[option] Y "});
        assert!(extract_questionnaire(&args).is_some());
    }

    #[test]
    fn extract_askuser_questions_structured() {
        let args = json!({
            "questions": [
                {
                    "question": "What writing style?",
                    "topic": "style",
                    "options": ["Formal", "Casual"]
                }
            ]
        });
        let result = extract_askuser_questions(&args);
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_array().unwrap().len(), 1);
    }

    #[test]
    fn extract_askuser_questions_rejects_single_option() {
        let args = json!({
            "questions": [{
                "question": "Q?",
                "topic": "t",
                "options": ["Only one"]
            }]
        });
        assert!(extract_askuser_questions(&args).is_none());
    }

    #[test]
    fn extract_askuser_questions_rejects_too_many() {
        let args = json!({
            "questions": [
                { "question": "Q1", "topic": "t1", "options": ["A", "B"] },
                { "question": "Q2", "topic": "t2", "options": ["A", "B"] },
                { "question": "Q3", "topic": "t3", "options": ["A", "B"] },
                { "question": "Q4", "topic": "t4", "options": ["A", "B"] },
                { "question": "Q5", "topic": "t5", "options": ["A", "B"] }
            ]
        });
        assert!(extract_askuser_questions(&args).is_none());
    }

    #[test]
    fn askuser_parser_allowlist_matches_registered_schema_properties() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();
        let schema = crate::agent_tools::registry::get_schema("askuser", &context)
            .expect("askuser schema should exist");
        let schema_fields: BTreeSet<String> = schema
            .get("properties")
            .and_then(|value| value.as_object())
            .expect("schema properties")
            .keys()
            .cloned()
            .collect();
        let parser_fields: BTreeSet<String> = parser_contract_fields()
            .iter()
            .map(|field| field.to_string())
            .collect();

        assert_eq!(schema_fields, parser_fields);
    }
}

