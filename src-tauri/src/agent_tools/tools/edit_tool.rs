use std::path::PathBuf;

use serde_json::json;

use crate::agent_tools::contracts::{EditInput, EditTarget};
use crate::application::command_usecases::chapter::update_chapter_metadata_usecase;
use crate::application::command_usecases::volume::update_volume_usecase;
use crate::models::jvm::Actor;
use crate::models::AppError;
use crate::services::jvm::{
    apply_edit_ops_to_doc, build_snapshot_id, commit_full_document, snapshot_id_matches,
};
use crate::services::{read_json, VcCommitPort, VersioningService};

const MANUSCRIPTS_DIR: &str = "manuscripts";

pub fn run(input: EditInput, call_id: &str) -> Result<serde_json::Value, AppError> {
    validate_edit_input(&input)?;

    let target = input.target.clone().unwrap_or(EditTarget::ChapterContent);
    match target {
        EditTarget::VolumeMeta => edit_volume_meta(input),
        EditTarget::ChapterMeta => edit_chapter_meta(input),
        EditTarget::ChapterContent => edit_chapter_content(input, call_id),
    }
}

fn edit_volume_meta(input: EditInput) -> Result<serde_json::Value, AppError> {
    let changed_fields = collect_changed_fields(
        [
            ("title", input.title.is_some()),
            ("summary", input.summary.is_some()),
        ]
        .into_iter(),
    );
    if changed_fields.is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: volume_meta edit requires at least one field",
        ));
    }

    if input.dry_run {
        return Ok(json!({
            "mode": "preview",
            "accepted": true,
            "path": input.path,
            "target": "volume_meta",
            "changed_fields": changed_fields,
        }));
    }

    let volume = update_volume_usecase(
        &input.project_path,
        input.path.trim(),
        input.title.clone(),
        input.summary.clone(),
    )?;

    Ok(json!({
        "mode": "commit",
        "accepted": true,
        "path": input.path,
        "target": "volume_meta",
        "changed_fields": changed_fields,
        "metadata": {
            "type": "volume",
            "volume_id": volume.volume_id,
            "title": volume.title,
            "summary": volume.summary,
            "updated_at": volume.updated_at,
        }
    }))
}

fn edit_chapter_meta(input: EditInput) -> Result<serde_json::Value, AppError> {
    let changed_fields = collect_changed_fields(
        [
            ("title", input.title.is_some()),
            ("summary", input.summary.is_some()),
            ("status", input.status.is_some()),
            ("target_words", input.target_words.is_some()),
            ("tags", input.tags.is_some()),
            ("pinned_assets", input.pinned_assets.is_some()),
        ]
        .into_iter(),
    );
    if changed_fields.is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: chapter_meta edit requires at least one field",
        ));
    }

    if input.dry_run {
        return Ok(json!({
            "mode": "preview",
            "accepted": true,
            "path": input.path,
            "target": "chapter_meta",
            "changed_fields": changed_fields,
        }));
    }

    let chapter = update_chapter_metadata_usecase(
        &input.project_path,
        input.path.trim(),
        input.title.clone(),
        input.summary.clone(),
        input.status.clone(),
        input.target_words,
        input.tags.clone(),
        input.pinned_assets.clone(),
    )?;

    Ok(json!({
        "mode": "commit",
        "accepted": true,
        "path": input.path,
        "target": "chapter_meta",
        "changed_fields": changed_fields,
        "metadata": {
            "type": "chapter",
            "chapter_id": chapter.id,
            "title": chapter.title,
            "summary": chapter.summary,
            "status": chapter.status.map(|s| format!("{:?}", s).to_lowercase()),
            "target_words": chapter.target_words,
            "tags": chapter.tags,
            "pinned_assets": chapter.pinned_assets,
            "updated_at": chapter.updated_at,
        }
    }))
}

fn collect_changed_fields<'a>(flags: impl Iterator<Item = (&'a str, bool)>) -> Vec<&'a str> {
    flags
        .filter_map(|(field, changed)| changed.then_some(field))
        .collect()
}

fn edit_chapter_content(input: EditInput, call_id: &str) -> Result<serde_json::Value, AppError> {
    let snapshot_id = input
        .snapshot_id
        .clone()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            AppError::invalid_argument("E_TOOL_SCHEMA_INVALID: snapshot_id is required")
        })?;

    if input.ops.is_empty() {
        return Err(AppError {
            code: crate::models::ErrorCode::InvalidArgument,
            message: "ops must contain at least one operation".to_string(),
            details: Some(json!({ "code": "E_EDIT_OPS_EMPTY" })),
            recoverable: Some(true),
        });
    }

    let chapter_path = input.path.trim().to_string();
    let full_path = PathBuf::from(&input.project_path)
        .join(MANUSCRIPTS_DIR)
        .join(&chapter_path);

    if !full_path.exists() {
        return Err(AppError::not_found(
            "E_TOOL_NOT_FOUND: chapter path not found",
        ));
    }

    let chapter: crate::models::Chapter = read_json(&full_path)?;

    let vc = VersioningService::new();
    let entity_id = format!("chapter:{}", chapter_path);
    let head = vc.get_current_head(&input.project_path, &entity_id)?;
    ensure_head_matches_base_revision(head.revision, input.base_revision)?;
    ensure_snapshot_matches_head(
        &chapter_path,
        head.revision,
        &head.json_hash,
        &snapshot_id,
        input.base_revision,
    )?;

    let applied = apply_edit_ops_to_doc(&chapter.content, &input.ops)?;
    let diagnostics = applied.diagnostics;

    let accepted = !diagnostics
        .iter()
        .any(|d| d.level == crate::models::DiagnosticLevel::Error);

    if input.dry_run {
        return Ok(json!({
            "mode": "preview",
            "accepted": accepted,
            "path": input.path,
            "target": "chapter_content",
            "revision_before": head.revision,
            "revision_after": head.revision,
            "diagnostics": diagnostics,
            "diff_summary": applied.diff_summary,
            "changed_block_ids": applied.changed_block_ids,
            "snapshot_id": snapshot_id,
            "hash_after": head.json_hash,
        }));
    }

    if !accepted {
        return Err(AppError::invalid_argument(
            "E_JVM_VALIDATION_FAIL: preview contains errors, commit aborted",
        ));
    }

    let commit = commit_full_document(
        &input.project_path,
        &chapter_path,
        input.base_revision as i64,
        call_id,
        map_actor(&input.actor),
        applied.doc,
        Vec::new(),
    )?;

    let next_snapshot_id = build_snapshot_id(
        &chapter_path,
        commit.revision_after,
        &commit.json_hash_after,
    );

    Ok(json!({
        "mode": "commit",
        "accepted": commit.ok,
        "path": input.path,
        "target": "chapter_content",
        "revision_before": commit.revision_before,
        "revision_after": commit.revision_after,
        "diagnostics": diagnostics,
        "diff_summary": applied.diff_summary,
        "changed_block_ids": applied.changed_block_ids,
        "snapshot_id": next_snapshot_id,
        "tx_id": commit.tx_id,
        "hash_after": commit.json_hash_after,
    }))
}

fn validate_edit_input(input: &EditInput) -> Result<(), AppError> {
    if input.project_path.trim().is_empty() || input.path.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: project_path/path are required",
        ));
    }
    Ok(())
}

fn ensure_head_matches_base_revision(
    current_revision: i64,
    base_revision: u64,
) -> Result<(), AppError> {
    let expected_revision = base_revision as i64;
    if current_revision == expected_revision {
        return Ok(());
    }

    Err(AppError {
        code: crate::models::ErrorCode::Conflict,
        message: format!(
            "Revision conflict: your base_revision ({}) does not match the current revision ({}). Another edit was applied since your last read. Please call 'read' to get the latest content and revision, then retry your edit with the updated base_revision.",
            base_revision, current_revision
        ),
        details: Some(json!({
            "code": "E_VC_CONFLICT_REVISION",
            "expected_revision": base_revision,
            "current_revision": current_revision,
        })),
        recoverable: Some(true),
    })
}

fn ensure_snapshot_matches_head(
    chapter_path: &str,
    current_revision: i64,
    current_hash: &str,
    provided_snapshot_id: &str,
    base_revision: u64,
) -> Result<(), AppError> {
    if snapshot_id_matches(
        chapter_path,
        current_revision,
        current_hash,
        provided_snapshot_id,
    ) {
        return Ok(());
    }

    let expected = build_snapshot_id(chapter_path, current_revision, current_hash);
    Err(AppError {
        code: crate::models::ErrorCode::Conflict,
        message: format!(
            "snapshot_id is stale: expected latest snapshot for revision {}",
            current_revision
        ),
        details: Some(json!({
            "code": "E_EDIT_SNAPSHOT_STALE",
            "path": chapter_path,
            "provided_snapshot_id": provided_snapshot_id,
            "expected_snapshot_id": expected,
            "current_revision": current_revision,
            "base_revision": base_revision,
        })),
        recoverable: Some(true),
    })
}

#[cfg(test)]
mod tests {
    use super::{ensure_head_matches_base_revision, ensure_snapshot_matches_head};

    #[test]
    fn conflict_error_contains_recovery_guidance() {
        let err = ensure_head_matches_base_revision(5, 3).expect_err("should conflict");
        assert!(err.message.contains("Revision conflict:"));
        assert!(err.message.contains("base_revision (3)"));
        assert!(err.message.contains("current revision (5)"));
        assert!(err
            .message
            .contains("Please call 'read' to get the latest content and revision"));
        assert_eq!(err.recoverable, Some(true));
        assert_eq!(
            err.details
                .as_ref()
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("E_VC_CONFLICT_REVISION")
        );
    }

    #[test]
    fn stale_snapshot_error_contains_expected_code() {
        let err = ensure_snapshot_matches_head(
            "vol_1/ch_1.json",
            10,
            "sha256:new",
            "snap:vol_1/ch_1.json:9:sha256:old",
            10,
        )
        .expect_err("should be stale");

        assert_eq!(
            err.details
                .as_ref()
                .and_then(|v| v.get("code"))
                .and_then(|v| v.as_str()),
            Some("E_EDIT_SNAPSHOT_STALE")
        );
    }
}

fn map_actor(actor: &crate::agent_tools::contracts::Actor) -> Actor {
    match actor {
        crate::agent_tools::contracts::Actor::Agent => Actor::Agent,
        crate::agent_tools::contracts::Actor::User => Actor::User,
        crate::agent_tools::contracts::Actor::System => Actor::System,
    }
}
