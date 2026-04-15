use std::path::PathBuf;

use serde_json::json;

use crate::agent_tools::runtime::{
    execute_context_read, execute_context_search, execute_draft_write, execute_knowledge_write,
    execute_structure_edit, execute_workspace_map,
};
use crate::models::{Chapter, ProjectMetadata, VolumeMetadata};
use crate::services::{ensure_dir, read_json, write_json};

fn create_temp_project() -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("magic_tool_runtime_test_{}", uuid::Uuid::new_v4()));
    ensure_dir(&root).expect("create temp project root");
    let project = ProjectMetadata::new("tool-test".to_string(), "tester".to_string(), None, None);
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
fn structure_edit_create_chapter_dry_run_no_side_effects() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");
    let vol_dir = project_path.join("manuscripts").join(&volume.volume_id);
    assert!(vol_dir.join("volume.json").exists());

    let input = json!({
        "op": "create",
        "node_type": "chapter",
        "parent_ref": volume_ref(&volume.volume_id),
        "title": "第一章",
        "dry_run": true
    });

    let result = execute_structure_edit(
        &project_path.to_string_lossy(),
        input,
        "call_test_create_dry".to_string(),
    );
    assert!(result.ok, "expected ok=true, got: {:?}", result.error);
    assert_eq!(
        result
            .data
            .as_ref()
            .and_then(|d| d.get("mode"))
            .and_then(|v| v.as_str()),
        Some("preview")
    );

    // No chapter files should be created in dry_run.
    let json_files = crate::services::list_files(&vol_dir, ".json").expect("list files");
    assert_eq!(json_files, vec!["volume.json"]);
}

#[test]
fn structure_edit_move_position_is_zero_based() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");

    let ch1 = create_chapter(&project_path, &volume.volume_id, "第一章");
    let ch2 = create_chapter(&project_path, &volume.volume_id, "第二章");

    let input = json!({
        "op": "move",
        "node_type": "chapter",
        "target_ref": chapter_ref(&volume.volume_id, &ch2.id),
        "parent_ref": volume_ref(&volume.volume_id),
        "position": 0
    });

    let result = execute_structure_edit(
        &project_path.to_string_lossy(),
        input,
        "call_test_move".to_string(),
    );
    assert!(result.ok, "expected ok=true, got: {:?}", result.error);

    let volume_meta: VolumeMetadata = read_json(
        &project_path
            .join("manuscripts")
            .join(&volume.volume_id)
            .join("volume.json"),
    )
    .expect("read volume.json");

    assert_eq!(
        volume_meta.chapter_order.first().cloned(),
        Some(ch2.id.clone())
    );
    assert!(volume_meta.chapter_order.iter().any(|id| id == &ch1.id));
}

#[test]
fn structure_edit_archive_and_restore_chapter_roundtrip() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");
    let chapter = create_chapter(&project_path, &volume.volume_id, "第一章");

    let cref = chapter_ref(&volume.volume_id, &chapter.id);
    let chapter_path = project_path
        .join("manuscripts")
        .join(&volume.volume_id)
        .join(format!("{}.json", chapter.id));
    assert!(chapter_path.exists());

    let archive = execute_structure_edit(
        &project_path.to_string_lossy(),
        json!({
            "op": "archive",
            "node_type": "chapter",
            "target_ref": cref,
            "dry_run": false
        }),
        "call_test_archive".to_string(),
    );
    assert!(
        archive.ok,
        "expected archive ok=true, got: {:?}",
        archive.error
    );
    assert!(
        !chapter_path.exists(),
        "chapter should be moved to recycle bin"
    );

    let restore = execute_structure_edit(
        &project_path.to_string_lossy(),
        json!({
            "op": "restore",
            "node_type": "chapter",
            "target_ref": chapter_ref(&volume.volume_id, &chapter.id),
            "dry_run": false
        }),
        "call_test_restore".to_string(),
    );
    assert!(
        restore.ok,
        "expected restore ok=true, got: {:?}",
        restore.error
    );
    assert!(chapter_path.exists(), "chapter should be restored");
}

#[test]
fn draft_write_rejects_payload_too_large() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");
    let chapter = create_chapter(&project_path, &volume.volume_id, "第一章");

    let too_large = "a".repeat(120_001);
    let result = execute_draft_write(
        &project_path.to_string_lossy(),
        json!({
            "target_ref": chapter_ref(&volume.volume_id, &chapter.id),
            "write_mode": "rewrite",
            "instruction": "test",
            "content": { "kind": "markdown", "value": too_large },
            "dry_run": true
        }),
        "call_test_payload".to_string(),
    );

    assert!(!result.ok);
    assert_eq!(
        result.error.as_ref().map(|e| e.code.as_str()),
        Some("E_PAYLOAD_TOO_LARGE")
    );
}

#[test]
fn draft_write_commit_updates_chapter_doc_and_revision_meta() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");
    let chapter = create_chapter(&project_path, &volume.volume_id, "第一章");

    let result = execute_draft_write(
        &project_path.to_string_lossy(),
        json!({
            "target_ref": chapter_ref(&volume.volume_id, &chapter.id),
            "write_mode": "rewrite",
            "instruction": "write a simple chapter",
            "content": { "kind": "markdown", "value": "# Hello\\n\\nWorld" },
            "dry_run": false,
            "idempotency_key": "k_test_commit"
        }),
        "call_test_commit".to_string(),
    );

    assert!(result.ok, "expected ok=true, got: {:?}", result.error);
    assert_eq!(
        result
            .data
            .as_ref()
            .and_then(|d| d.get("mode"))
            .and_then(|v| v.as_str()),
        Some("commit")
    );
    assert!(result.meta.revision_before.is_some());
    assert!(result.meta.revision_after.is_some());
    assert!(
        result.meta.revision_after.unwrap() >= result.meta.revision_before.unwrap(),
        "revision should not decrease"
    );

    let chapter_path = project_path
        .join("manuscripts")
        .join(&volume.volume_id)
        .join(format!("{}.json", chapter.id));
    let updated: Chapter = read_json(&chapter_path).expect("read updated chapter");
    let blocks = updated
        .content
        .get("content")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!blocks.is_empty(), "chapter doc should have blocks");
    assert_eq!(
        blocks[0].get("type").and_then(|v| v.as_str()),
        Some("heading")
    );
}

#[test]
fn legacy_timeout_ms_is_accepted_for_backward_compatibility() {
    let project_path = create_temp_project();
    let volume = create_volume(&project_path, "卷一");
    let chapter = create_chapter(&project_path, &volume.volume_id, "第一章");

    let workspace_map = execute_workspace_map(
        &project_path.to_string_lossy(),
        json!({
            "scope": "book",
            "timeout_ms": 1500
        }),
        "call_test_workspace_timeout".to_string(),
    );
    assert!(
        workspace_map.ok,
        "workspace_map should accept legacy timeout_ms: {:?}",
        workspace_map.error
    );

    let context_read = execute_context_read(
        &project_path.to_string_lossy(),
        json!({
            "target_ref": chapter_ref(&volume.volume_id, &chapter.id),
            "timeout_ms": 1500
        }),
        "call_test_context_read_timeout".to_string(),
    );
    assert!(
        context_read.ok,
        "context_read should accept legacy timeout_ms: {:?}",
        context_read.error
    );

    let context_search = execute_context_search(
        &project_path.to_string_lossy(),
        json!({
            "query": "第一章",
            "timeout_ms": 1500
        }),
        "call_test_context_search_timeout".to_string(),
    );
    assert!(
        context_search.ok,
        "context_search should accept legacy timeout_ms: {:?}",
        context_search.error
    );

    let draft_write = execute_draft_write(
        &project_path.to_string_lossy(),
        json!({
            "target_ref": chapter_ref(&volume.volume_id, &chapter.id),
            "write_mode": "rewrite",
            "instruction": "rewrite softly",
            "content": { "kind": "markdown", "value": "# 标题\n\n正文" },
            "dry_run": true,
            "timeout_ms": 1500
        }),
        "call_test_draft_timeout".to_string(),
    );
    assert!(
        draft_write.ok,
        "draft_write should accept legacy timeout_ms: {:?}",
        draft_write.error
    );

    let structure_edit = execute_structure_edit(
        &project_path.to_string_lossy(),
        json!({
            "op": "create",
            "node_type": "chapter",
            "parent_ref": volume_ref(&volume.volume_id),
            "title": "新章",
            "dry_run": true,
            "timeout_ms": 1500
        }),
        "call_test_structure_timeout".to_string(),
    );
    assert!(
        structure_edit.ok,
        "structure_edit should accept legacy timeout_ms: {:?}",
        structure_edit.error
    );
}

#[test]
fn structure_edit_rejects_knowledge_item_at_schema_boundary() {
    let project_path = create_temp_project();

    let result = execute_structure_edit(
        &project_path.to_string_lossy(),
        json!({
            "op": "create",
            "node_type": "knowledge_item",
            "title": "设定项"
        }),
        "call_test_structure_knowledge_item".to_string(),
    );

    assert!(!result.ok);
    assert_eq!(
        result.error.as_ref().map(|err| err.code.as_str()),
        Some("E_TOOL_SCHEMA_INVALID")
    );
    assert!(result
        .error
        .as_ref()
        .and_then(|err| Some(err.message.contains("knowledge_item")))
        .unwrap_or(false));
}

#[test]
fn knowledge_write_reports_nested_fields_path_for_non_object_fields() {
    let project_path = create_temp_project();

    let result = execute_knowledge_write(
        &project_path.to_string_lossy(),
        json!({
            "op": "propose",
            "changes": [{
                "target_ref": "knowledge:.magic_novel/terms/foo.json",
                "kind": "add",
                "fields": "summary = foo"
            }],
            "dry_run": true
        }),
        "call_test_knowledge_write_fields_path".to_string(),
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
