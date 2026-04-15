use std::path::PathBuf;

use serde_json::json;

use super::{execute_draft_write, execute_knowledge_write, execute_structure_edit};
use crate::models::{Chapter, ProjectMetadata, VolumeMetadata};
use crate::services::{ensure_dir, write_json};

fn create_temp_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "magic_tool_contract_regression_{}",
        uuid::Uuid::new_v4()
    ));
    ensure_dir(&root).expect("create temp project root");
    let project =
        ProjectMetadata::new("tool-contract".to_string(), "tester".to_string(), None, None);
    write_json(&root.join("project.json"), &project).expect("write project.json");
    root
}

fn create_volume(project_path: &PathBuf, title: &str) -> VolumeMetadata {
    crate::application::command_usecases::volume::create_volume_usecase(
        &project_path.to_string_lossy(),
        title,
    )
    .expect("create volume")
}

fn create_chapter(project_path: &PathBuf, volume_id: &str, title: &str) -> Chapter {
    crate::application::command_usecases::chapter::create_chapter_usecase(
        &project_path.to_string_lossy(),
        volume_id,
        title,
    )
    .expect("create chapter")
}

fn volume_ref(volume_id: &str) -> String {
    format!("volume:manuscripts/{volume_id}/volume.json")
}

fn chapter_ref(volume_id: &str, chapter_id: &str) -> String {
    format!("chapter:manuscripts/{volume_id}/{chapter_id}.json")
}

#[test]
fn knowledge_write_accepts_minimal_preview_contract_input() {
    let project_path = create_temp_project();

    let result = execute_knowledge_write(
        &project_path.to_string_lossy(),
        json!({
            "op": "propose",
            "changes": [{
                "target_ref": "knowledge:.magic_novel/terms/story_hook.json",
                "kind": "add",
                "fields": {
                    "summary": "开篇伏笔"
                }
            }],
            "dry_run": true
        }),
        "call_contract_knowledge_ok".to_string(),
    );

    assert!(result.ok, "expected ok=true, got: {:?}", result.error);
    assert_eq!(
        result
            .data
            .as_ref()
            .and_then(|data| data.get("status"))
            .and_then(|value| value.as_str()),
        Some("proposed")
    );
}

#[test]
fn knowledge_write_rejects_missing_fields_in_contract_regression_suite() {
    let project_path = create_temp_project();

    let result = execute_knowledge_write(
        &project_path.to_string_lossy(),
        json!({
            "op": "propose",
            "changes": [{
                "target_ref": "knowledge:.magic_novel/terms/story_hook.json",
                "kind": "add"
            }],
            "dry_run": true
        }),
        "call_contract_knowledge_missing_fields".to_string(),
    );

    assert!(!result.ok);
    assert_eq!(
        result.error.as_ref().map(|err| err.code.as_str()),
        Some("E_TOOL_SCHEMA_INVALID")
    );
    assert_eq!(
        result.error.as_ref().map(|err| err.message.as_str()),
        Some("changes[0].fields is required")
    );
}

#[test]
fn knowledge_write_rejects_wrong_fields_type_in_contract_regression_suite() {
    let project_path = create_temp_project();

    let result = execute_knowledge_write(
        &project_path.to_string_lossy(),
        json!({
            "op": "propose",
            "changes": [{
                "target_ref": "knowledge:.magic_novel/terms/story_hook.json",
                "kind": "add",
                "fields": "summary = 开篇伏笔"
            }],
            "dry_run": true
        }),
        "call_contract_knowledge_bad_fields".to_string(),
    );

    assert!(!result.ok);
    assert_eq!(
        result.error.as_ref().map(|err| err.code.as_str()),
        Some("E_TOOL_SCHEMA_INVALID")
    );
    assert_eq!(
        result.error.as_ref().map(|err| err.message.as_str()),
        Some("changes[0].fields must be an object, got string")
    );
}

#[test]
fn draft_write_accepts_minimal_preview_contract_input() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");
    let chapter = create_chapter(&project_path, &volume.volume_id, "第一章");

    let result = execute_draft_write(
        &project_path.to_string_lossy(),
        json!({
            "target_ref": chapter_ref(&volume.volume_id, &chapter.id),
            "write_mode": "rewrite",
            "instruction": "整理章节开头",
            "content": { "kind": "markdown", "value": "# 第一章\n\n测试正文" },
            "dry_run": true
        }),
        "call_contract_draft_ok".to_string(),
    );

    assert!(result.ok, "expected ok=true, got: {:?}", result.error);
    assert_eq!(
        result
            .data
            .as_ref()
            .and_then(|data| data.get("mode"))
            .and_then(|value| value.as_str()),
        Some("preview")
    );
}

#[test]
fn draft_write_rejects_unknown_field_in_contract_regression_suite() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");
    let chapter = create_chapter(&project_path, &volume.volume_id, "第一章");

    let result = execute_draft_write(
        &project_path.to_string_lossy(),
        json!({
            "target_ref": chapter_ref(&volume.volume_id, &chapter.id),
            "write_mode": "rewrite",
            "instruction": "整理章节开头",
            "content": { "kind": "markdown", "value": "# 第一章\n\n测试正文" },
            "dry_run": true,
            "snapshot_id": "legacy_snapshot"
        }),
        "call_contract_draft_unknown_field".to_string(),
    );

    assert!(!result.ok);
    assert_eq!(
        result.error.as_ref().map(|err| err.code.as_str()),
        Some("E_TOOL_UNKNOWN_FIELD")
    );
    assert!(result
        .error
        .as_ref()
        .map(|err| err.message.contains("unknown field `snapshot_id`"))
        .unwrap_or(false));
}

#[test]
fn structure_edit_accepts_minimal_preview_contract_input() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");

    let result = execute_structure_edit(
        &project_path.to_string_lossy(),
        json!({
            "op": "create",
            "node_type": "chapter",
            "parent_ref": volume_ref(&volume.volume_id),
            "title": "新章",
            "dry_run": true
        }),
        "call_contract_structure_ok".to_string(),
    );

    assert!(result.ok, "expected ok=true, got: {:?}", result.error);
    assert_eq!(
        result
            .data
            .as_ref()
            .and_then(|data| data.get("mode"))
            .and_then(|value| value.as_str()),
        Some("preview")
    );
}

#[test]
fn structure_edit_rejects_unimplemented_capability_in_contract_regression_suite() {
    let project_path = create_temp_project();

    let result = execute_structure_edit(
        &project_path.to_string_lossy(),
        json!({
            "op": "create",
            "node_type": "knowledge_item",
            "title": "设定项"
        }),
        "call_contract_structure_unimplemented".to_string(),
    );

    assert!(!result.ok);
    assert_eq!(
        result.error.as_ref().map(|err| err.code.as_str()),
        Some("E_TOOL_SCHEMA_INVALID")
    );
    assert!(result
        .error
        .as_ref()
        .map(|err| err.message.contains("knowledge_item"))
        .unwrap_or(false));
}
