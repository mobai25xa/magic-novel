use serde_json::Value;

use super::super::types::ToolCallInfo;

const ASKUSER_FIELDS: &[&str] = &["questions", "questionnaire"];
const ASKUSER_QUESTION_FIELDS: &[&str] = &["question", "topic", "options"];
const ASKUSER_MIN_QUESTIONS: usize = 1;
const ASKUSER_MAX_QUESTIONS: usize = 4;
const ASKUSER_MIN_OPTIONS: usize = 2;
const ASKUSER_MAX_OPTIONS: usize = 4;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parser_contract_fields() -> &'static [&'static str] {
    ASKUSER_FIELDS
}

pub(crate) fn is_askuser_call(tc: &ToolCallInfo) -> bool {
    tc.tool_name.as_str() == "askuser"
}

pub(crate) fn extract_questionnaire(tc: &ToolCallInfo) -> Option<String> {
    let raw = tc.args.get("questionnaire")?;
    validate_questionnaire_value(raw).ok()?;
    Some(raw.as_str()?.trim().to_string())
}

pub(crate) fn validate_askuser_args(args: &Value) -> Result<(), String> {
    if !args.is_object() {
        return Err("askuser args must be an object".to_string());
    }

    reject_unknown_fields(args, ASKUSER_FIELDS)?;

    let mut has_questions = false;
    if let Some(questions) = args.get("questions") {
        validate_questions_value(questions)?;
        has_questions = true;
    }

    let mut has_questionnaire = false;
    if let Some(questionnaire) = args.get("questionnaire") {
        validate_questionnaire_value(questionnaire)?;
        has_questionnaire = true;
    }

    if !has_questions && !has_questionnaire {
        return Err("askuser args: requires questions or questionnaire".to_string());
    }

    Ok(())
}

/// Extract structured questions from askuser tool call args.
/// Returns a validated JSON array value for the event payload.
pub(crate) fn extract_askuser_questions(tc: &ToolCallInfo) -> Option<serde_json::Value> {
    let questions = tc.args.get("questions")?;
    validate_questions_value(questions).ok()?;
    Some(questions.clone())
}

fn validate_questionnaire_value(value: &Value) -> Result<(), String> {
    let Some(questionnaire) = value.as_str() else {
        return Err("askuser questionnaire must be a non-empty string".to_string());
    };

    if questionnaire.trim().is_empty() {
        return Err("askuser questionnaire must be a non-empty string".to_string());
    }

    Ok(())
}

fn validate_questions_value(value: &Value) -> Result<(), String> {
    let Some(questions) = value.as_array() else {
        return Err("askuser questions must be an array".to_string());
    };

    if !(ASKUSER_MIN_QUESTIONS..=ASKUSER_MAX_QUESTIONS).contains(&questions.len()) {
        return Err(format!(
            "askuser questions must contain between {ASKUSER_MIN_QUESTIONS} and {ASKUSER_MAX_QUESTIONS} items"
        ));
    }

    for (index, question) in questions.iter().enumerate() {
        validate_question_value(question, index)?;
    }

    Ok(())
}

fn validate_question_value(value: &Value, index: usize) -> Result<(), String> {
    let Some(question) = value.as_object() else {
        return Err(format!("askuser questions[{index}] must be an object"));
    };

    reject_unknown_question_fields(question, index)?;
    validate_non_empty_string_field(
        question,
        "question",
        &format!("questions[{index}].question"),
    )?;
    validate_non_empty_string_field(question, "topic", &format!("questions[{index}].topic"))?;
    validate_options_value(question.get("options"), index)?;
    Ok(())
}

fn reject_unknown_question_fields(
    question: &serde_json::Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    for key in question.keys() {
        if !ASKUSER_QUESTION_FIELDS.contains(&key.as_str()) {
            return Err(format!("askuser questions[{index}]: unknown field '{key}'"));
        }
    }

    Ok(())
}

fn validate_non_empty_string_field(
    question: &serde_json::Map<String, Value>,
    field: &str,
    path: &str,
) -> Result<(), String> {
    let Some(value) = question.get(field).and_then(|value| value.as_str()) else {
        return Err(format!("askuser {path} must be a non-empty string"));
    };

    if value.trim().is_empty() {
        return Err(format!("askuser {path} must be a non-empty string"));
    }

    Ok(())
}

fn validate_options_value(option_value: Option<&Value>, index: usize) -> Result<(), String> {
    let path = format!("questions[{index}].options");
    let Some(options) = option_value.and_then(|value| value.as_array()) else {
        return Err(format!("askuser {path} must be an array"));
    };

    if !(ASKUSER_MIN_OPTIONS..=ASKUSER_MAX_OPTIONS).contains(&options.len()) {
        return Err(format!(
            "askuser {path} must contain between {ASKUSER_MIN_OPTIONS} and {ASKUSER_MAX_OPTIONS} items"
        ));
    }

    for (option_index, option) in options.iter().enumerate() {
        let Some(label) = option.as_str() else {
            return Err(format!(
                "askuser {path}[{option_index}] must be a non-empty string"
            ));
        };

        if label.trim().is_empty() {
            return Err(format!(
                "askuser {path}[{option_index}] must be a non-empty string"
            ));
        }
    }

    Ok(())
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
    fn validate_askuser_args_requires_object_payload() {
        let err = validate_askuser_args(&json!("bad payload")).expect_err("should fail");
        assert_eq!(err, "askuser args must be an object");
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
    fn validate_askuser_args_rejects_invalid_nested_questions() {
        let err = validate_askuser_args(&json!({
            "questions": [{
                "question": "Continue?",
                "topic": "confirm",
                "options": ["Only one"]
            }]
        }))
        .expect_err("should fail");
        assert_eq!(
            err,
            "askuser questions[0].options must contain between 2 and 4 items"
        );
    }

    #[test]
    fn validate_askuser_args_rejects_invalid_questions_even_with_questionnaire_fallback() {
        let err = validate_askuser_args(&json!({
            "questions": [{
                "question": "Continue?",
                "topic": "",
                "options": ["Yes", "No"]
            }],
            "questionnaire": "1. [question] Continue?\n[topic] confirm\n[option] Yes\n[option] No"
        }))
        .expect_err("should fail");
        assert_eq!(err, "askuser questions[0].topic must be a non-empty string");
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

    #[test]
    fn askuser_nested_question_contract_matches_registered_schema() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();
        let schema = crate::agent_tools::registry::get_schema("askuser", &context)
            .expect("askuser schema should exist");
        let questions_schema = schema
            .get("properties")
            .and_then(|value| value.get("questions"))
            .expect("questions schema");
        assert_eq!(
            questions_schema
                .get("minItems")
                .and_then(|value| value.as_u64()),
            Some(ASKUSER_MIN_QUESTIONS as u64)
        );
        assert_eq!(
            questions_schema
                .get("maxItems")
                .and_then(|value| value.as_u64()),
            Some(ASKUSER_MAX_QUESTIONS as u64)
        );

        let item_schema = questions_schema.get("items").expect("question item schema");
        let schema_fields: BTreeSet<String> = item_schema
            .get("properties")
            .and_then(|value| value.as_object())
            .expect("question properties")
            .keys()
            .cloned()
            .collect();
        let required_fields: BTreeSet<String> = item_schema
            .get("required")
            .and_then(|value| value.as_array())
            .expect("required fields")
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect();
        let parser_fields: BTreeSet<String> = ASKUSER_QUESTION_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect();
        assert_eq!(schema_fields, parser_fields);
        assert_eq!(required_fields, parser_fields);
        assert_eq!(
            item_schema
                .get("additionalProperties")
                .and_then(|value| value.as_bool()),
            Some(false)
        );

        let options_schema = item_schema
            .get("properties")
            .and_then(|value| value.get("options"))
            .expect("options schema");
        assert_eq!(
            options_schema
                .get("minItems")
                .and_then(|value| value.as_u64()),
            Some(ASKUSER_MIN_OPTIONS as u64)
        );
        assert_eq!(
            options_schema
                .get("maxItems")
                .and_then(|value| value.as_u64()),
            Some(ASKUSER_MAX_OPTIONS as u64)
        );
    }
}
