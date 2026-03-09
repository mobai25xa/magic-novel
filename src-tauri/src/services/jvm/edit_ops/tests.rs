use crate::agent_tools::contracts::{EditOp, SnapshotBlockInput};

use super::apply_edit_ops_to_doc;

fn base_doc() -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": [
            {"type": "paragraph", "attrs": {"id": "p1"}, "content": [{"type": "text", "text": "A"}]},
            {"type": "paragraph", "attrs": {"id": "p2"}, "content": [{"type": "text", "text": "B"}]}
        ]
    })
}

#[test]
fn replace_block_updates_target() {
    let result = apply_edit_ops_to_doc(
        &base_doc(),
        &[EditOp::ReplaceBlock {
            block_id: "p1".to_string(),
            markdown: "Changed".to_string(),
        }],
    )
    .expect("replace should succeed");

    let as_text = result.doc.to_string();
    assert!(as_text.contains("Changed"));
}

#[test]
fn insert_after_appends_after_anchor() {
    let result = apply_edit_ops_to_doc(
        &base_doc(),
        &[EditOp::InsertAfter {
            block_id: "p1".to_string(),
            blocks: vec![SnapshotBlockInput {
                markdown: "After p1".to_string(),
            }],
        }],
    )
    .expect("insert_after should succeed");

    let content = result
        .doc
        .get("content")
        .and_then(|v| v.as_array())
        .expect("content array");
    assert_eq!(content.len(), 3);
    assert_eq!(
        content[1]
            .get("content")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("text"))
            .and_then(|v| v.as_str()),
        Some("After p1")
    );
}

#[test]
fn delete_block_removes_target() {
    let result = apply_edit_ops_to_doc(
        &base_doc(),
        &[EditOp::DeleteBlock {
            block_id: "p1".to_string(),
        }],
    )
    .expect("delete should succeed");

    let content = result
        .doc
        .get("content")
        .and_then(|v| v.as_array())
        .expect("content array");
    assert_eq!(content.len(), 1);
}

#[test]
fn replace_range_replaces_expected_span() {
    let result = apply_edit_ops_to_doc(
        &base_doc(),
        &[EditOp::ReplaceRange {
            start_block_id: "p1".to_string(),
            end_block_id: "p2".to_string(),
            blocks: vec![SnapshotBlockInput {
                markdown: "Merged".to_string(),
            }],
        }],
    )
    .expect("replace range should succeed");

    let content = result
        .doc
        .get("content")
        .and_then(|v| v.as_array())
        .expect("content array");
    assert_eq!(content.len(), 1);
    assert_eq!(
        content[0]
            .get("content")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("text"))
            .and_then(|v| v.as_str()),
        Some("Merged")
    );
}

#[test]
fn empty_ops_returns_expected_error() {
    let err = apply_edit_ops_to_doc(&base_doc(), &[]).expect_err("should fail");
    let code = err
        .details
        .as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str());
    assert_eq!(code, Some("E_EDIT_OPS_EMPTY"));
}

#[test]
fn missing_block_returns_expected_error() {
    let err = apply_edit_ops_to_doc(
        &base_doc(),
        &[EditOp::DeleteBlock {
            block_id: "missing".to_string(),
        }],
    )
    .expect_err("should fail");

    let code = err
        .details
        .as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str());
    assert_eq!(code, Some("E_EDIT_BLOCK_NOT_FOUND"));
}

#[test]
fn replace_range_invalid_order_returns_expected_error() {
    let err = apply_edit_ops_to_doc(
        &base_doc(),
        &[EditOp::ReplaceRange {
            start_block_id: "p2".to_string(),
            end_block_id: "p1".to_string(),
            blocks: vec![SnapshotBlockInput {
                markdown: "Merged".to_string(),
            }],
        }],
    )
    .expect_err("should fail");

    let code = err
        .details
        .as_ref()
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str());
    assert_eq!(code, Some("E_EDIT_RANGE_INVALID"));
}
