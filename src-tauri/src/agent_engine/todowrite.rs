//! Agent Engine - TodoWrite tool parsing

use serde::{Deserialize, Serialize};
use serde_json::Value;

const TODOWRITE_FIELDS: &[&str] = &["todos"];
const TODOWRITE_ITEM_FIELDS: &[&str] = &["status", "text"];

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parser_contract_fields() -> &'static [&'static str] {
    TODOWRITE_FIELDS
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parser_contract_item_fields() -> &'static [&'static str] {
    TODOWRITE_ITEM_FIELDS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub status: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoState {
    pub items: Vec<TodoItem>,
    pub last_updated_at: i64,
    pub source_call_id: Option<String>,
}

pub fn parse_todo_input(args: &Value, call_id: &str) -> Result<TodoState, String> {
    reject_unknown_fields(args, TODOWRITE_FIELDS, "todowrite")?;

    let todos = args
        .get("todos")
        .ok_or_else(|| "todowrite args: missing 'todos' field".to_string())?;

    let items_arr = todos
        .as_array()
        .ok_or_else(|| "todowrite args: 'todos' must be an array".to_string())?;

    if items_arr.len() > 50 {
        return Err("todowrite args: todos must contain at most 50 items".to_string());
    }

    let mut items = Vec::new();
    let mut in_progress_count = 0usize;
    for (i, item) in items_arr.iter().enumerate() {
        reject_unknown_fields(item, TODOWRITE_ITEM_FIELDS, &format!("todowrite item[{i}]"))?;

        let status = item
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string();

        if !matches!(status.as_str(), "pending" | "in_progress" | "completed") {
            return Err(format!(
                "todowrite args: item[{}].status must be 'pending', 'in_progress', or 'completed', got '{}'",
                i, status
            ));
        }

        let text = item
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("todowrite args: item[{}].text is required", i))?
            .trim()
            .to_string();

        if text.is_empty() {
            return Err(format!(
                "todowrite args: item[{}].text must not be empty",
                i
            ));
        }

        if text.chars().count() > 500 {
            return Err(format!(
                "todowrite args: item[{}].text must be <= 500 characters",
                i
            ));
        }

        if status == "in_progress" {
            in_progress_count += 1;
        }

        items.push(TodoItem { status, text });
    }

    if in_progress_count > 1 {
        return Err("todowrite args: at most one item can be in_progress".to_string());
    }

    Ok(TodoState {
        items,
        last_updated_at: chrono::Utc::now().timestamp_millis(),
        source_call_id: Some(call_id.to_string()),
    })
}

fn reject_unknown_fields(args: &Value, fields: &[&str], scope: &str) -> Result<(), String> {
    let Some(map) = args.as_object() else {
        return Ok(());
    };

    for key in map.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(format!("{scope}: unknown field '{key}'"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_valid_todos() {
        let args = json!({
            "todos": [
                { "status": "completed", "text": "Read chapter 1" },
                { "status": "in_progress", "text": "Edit chapter 2" },
                { "status": "pending", "text": "Write chapter 3" },
            ]
        });
        let state = parse_todo_input(&args, "call_1").unwrap();
        assert_eq!(state.items.len(), 3);
        assert_eq!(state.items[0].status, "completed");
        assert_eq!(state.items[1].text, "Edit chapter 2");
        assert!(state.source_call_id.as_deref() == Some("call_1"));
    }

    #[test]
    fn test_parse_missing_todos() {
        let args = json!({});
        assert!(parse_todo_input(&args, "call_1").is_err());
    }

    #[test]
    fn test_parse_invalid_status() {
        let args = json!({
            "todos": [{ "status": "unknown", "text": "test" }]
        });
        assert!(parse_todo_input(&args, "call_1").is_err());
    }

    #[test]
    fn test_parse_empty_text() {
        let args = json!({
            "todos": [{ "status": "pending", "text": "" }]
        });
        assert!(parse_todo_input(&args, "call_1").is_err());
    }

    #[test]
    fn test_parse_rejects_unknown_worker_field() {
        let args = json!({
            "todos": [
                { "status": "pending", "text": "Plan outline", "worker": "plot-architect" },
                { "status": "pending", "text": "Write prose" },
            ]
        });
        let err = parse_todo_input(&args, "call_1").expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("worker"));
    }

    #[test]
    fn test_parse_rejects_multiple_in_progress() {
        let args = json!({
            "todos": [
                { "status": "in_progress", "text": "task 1" },
                { "status": "in_progress", "text": "task 2" }
            ]
        });
        let err = parse_todo_input(&args, "call_1").expect_err("should fail");
        assert!(err.contains("at most one item"));
    }

    #[test]
    fn test_parse_rejects_too_many_items() {
        let many = (0..51)
            .map(|i| json!({ "status": "pending", "text": format!("task-{i}") }))
            .collect::<Vec<_>>();
        let args = json!({ "todos": many });
        let err = parse_todo_input(&args, "call_1").expect_err("should fail");
        assert!(err.contains("at most 50"));
    }

    #[test]
    fn test_parse_rejects_item_text_over_limit() {
        let args = json!({
            "todos": [{ "status": "pending", "text": "a".repeat(501) }]
        });
        let err = parse_todo_input(&args, "call_1").expect_err("should fail");
        assert!(err.contains("<= 500 characters"));
    }

    #[test]
    fn test_parse_rejects_unknown_top_level_fields() {
        let args = json!({
            "todos": [{ "status": "pending", "text": "task" }],
            "unexpected": true
        });
        let err = parse_todo_input(&args, "call_1").expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("unexpected"));
    }

    #[test]
    fn test_parse_rejects_unknown_item_fields() {
        let args = json!({
            "todos": [{ "status": "pending", "text": "task", "extra": "value" }]
        });
        let err = parse_todo_input(&args, "call_1").expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("extra"));
    }

    #[test]
    fn todowrite_parser_matches_live_schema_contract() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();
        let schema = crate::agent_tools::registry::get_schema("todowrite", &context)
            .expect("todowrite schema should exist");
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

        let item_fields: BTreeSet<String> = schema
            .get("properties")
            .and_then(|value| value.get("todos"))
            .and_then(|value| value.get("items"))
            .and_then(|value| value.get("properties"))
            .and_then(|value| value.as_object())
            .expect("item properties")
            .keys()
            .cloned()
            .collect();
        let parser_item_fields: BTreeSet<String> = parser_contract_item_fields()
            .iter()
            .map(|field| field.to_string())
            .collect();
        assert_eq!(item_fields, parser_item_fields);
    }
}
