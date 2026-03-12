use crate::review::types::ReviewRunInput;

pub(super) const REVIEW_CHECK_FIELDS: &[&str] = &[
    "scope_ref",
    "target_refs",
    "review_types",
    "branch_id",
    "task_card_ref",
    "context_pack_ref",
    "effective_rules_fingerprint",
    "severity_threshold",
];

pub(super) fn parse_review_check_input(args: &serde_json::Value) -> Result<ReviewRunInput, String> {
    reject_unknown_fields(args, REVIEW_CHECK_FIELDS, "review_check")?;

    serde_json::from_value::<ReviewRunInput>(args.clone())
        .map_err(|error| format!("review_check args: {error}"))
}

fn reject_unknown_fields(
    args: &serde_json::Value,
    allowed_fields: &[&str],
    tool_name: &str,
) -> Result<(), String> {
    let Some(map) = args.as_object() else {
        return Ok(());
    };

    for key in map.keys() {
        if !allowed_fields.contains(&key.as_str()) {
            return Err(format!("{tool_name} args: unknown field '{key}'"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    #[test]
    fn parse_review_check_accepts_full_payload() {
        let args = json!({
            "scope_ref": "chapter:manuscripts/vol_1/ch_1.json",
            "target_refs": ["manuscripts/vol_1/ch_1.json"],
            "review_types": ["word_count", "continuity"],
            "branch_id": "branch/main",
            "task_card_ref": "task:123",
            "context_pack_ref": "ctx:abc",
            "effective_rules_fingerprint": "rules:v1",
            "severity_threshold": "warn"
        });

        let input = parse_review_check_input(&args).expect("review_check parsed");
        assert_eq!(input.scope_ref, "chapter:manuscripts/vol_1/ch_1.json");
        assert_eq!(input.target_refs, vec!["manuscripts/vol_1/ch_1.json"]);
        assert_eq!(input.review_types.len(), 2);
        assert_eq!(input.severity_threshold.as_deref(), Some("warn"));
    }

    #[test]
    fn parse_review_check_rejects_unknown_fields() {
        let args = json!({
            "scope_ref": "chapter:manuscripts/vol_1/ch_1.json",
            "target_refs": ["manuscripts/vol_1/ch_1.json"],
            "unexpected": true
        });

        let err = parse_review_check_input(&args).expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("unexpected"));
    }

    #[test]
    fn parser_allowlist_matches_registered_schema_properties() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();
        let schema = crate::agent_tools::registry::get_schema("review_check", &context)
            .expect("review_check schema should exist");
        let schema_fields: BTreeSet<String> = schema
            .get("properties")
            .and_then(|value| value.as_object())
            .expect("schema properties")
            .keys()
            .cloned()
            .collect();
        let parser_fields: BTreeSet<String> = REVIEW_CHECK_FIELDS
            .iter()
            .map(|field| field.to_string())
            .collect();

        assert_eq!(schema_fields, parser_fields);
    }
}
