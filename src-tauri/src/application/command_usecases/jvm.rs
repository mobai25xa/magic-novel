use std::path::PathBuf;

use tauri::command;

use crate::models::jvm::{
    CommitRequest, CommitResult, ExportRequest, ExportResult, PreviewRequest, PreviewResult,
};
use crate::models::{AppError, Chapter, DiagnosticLevel};
use crate::services::jvm::{
    commit_patch_with_occ, ensure_doc_block_ids, export_chapter_to_markdown, generate_patch_ops,
    parse_markdown_to_blocks, validate_markdown_blocks, validate_patch_ops,
};
use crate::services::{list_files, read_json, write_json, VcCommitPort, VersioningService};

use crate::services::jvm::parser::app_err_jvm;

const MANUSCRIPTS_DIR: &str = "manuscripts";

#[command]
pub async fn jvm_export_chapter(input: ExportRequest) -> Result<ExportResult, AppError> {
    if input.call_id.trim().is_empty() {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "call_id is required".to_string(),
            false,
            None,
        ));
    }

    let full_path = PathBuf::from(&input.project_path)
        .join(MANUSCRIPTS_DIR)
        .join(&input.chapter_path);

    if !full_path.exists() {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "chapter_path not found".to_string(),
            false,
            None,
        ));
    }

    let chapter: Chapter = read_json(&full_path)?;

    // Get revision/hash from VC head.
    let vc = VersioningService::new();
    let entity_id = format!("chapter:{}", input.chapter_path);
    let head = vc.get_current_head(&input.project_path, &entity_id)?;

    let include_hints = input.include_block_hints.unwrap_or(false);

    export_chapter_to_markdown(&chapter, head.revision, head.json_hash, include_hints)
}

#[command]
pub async fn jvm_preview_patch(input: PreviewRequest) -> Result<PreviewResult, AppError> {
    if input.call_id.trim().is_empty() {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "call_id is required".to_string(),
            false,
            None,
        ));
    }

    let full_path = PathBuf::from(&input.project_path)
        .join(MANUSCRIPTS_DIR)
        .join(&input.chapter_path);

    if !full_path.exists() {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "chapter_path not found".to_string(),
            false,
            None,
        ));
    }

    // Load base chapter.
    let chapter: Chapter = read_json(&full_path)?;

    // OCC check against VC head, but preview never writes.
    let vc = VersioningService::new();
    let entity_id = format!("chapter:{}", input.chapter_path);
    let head = vc.get_current_head(&input.project_path, &entity_id)?;

    if head.revision != input.base_revision {
        return Err(app_err_jvm(
            "E_JVM_CONFLICT_REVISION",
            format!(
                "base_revision {} does not match current {}",
                input.base_revision, head.revision
            ),
            true,
            Some(serde_json::json!({
                "expected_revision": input.base_revision,
                "current_revision": head.revision,
            })),
        ));
    }

    let (md_blocks, mut diagnostics) = parse_markdown_to_blocks(&input.markdown)?;

    diagnostics.extend(validate_markdown_blocks(md_blocks.len()));

    let (patch_ops, patch_diags, diff_summary) = generate_patch_ops(&chapter.content, &md_blocks);
    diagnostics.extend(patch_diags);

    let patch_validation = validate_patch_ops(&patch_ops);
    diagnostics.extend(patch_validation);

    let ok = !diagnostics
        .iter()
        .any(|d| d.level == DiagnosticLevel::Error);

    Ok(PreviewResult {
        ok,
        patch_ops,
        diagnostics,
        diff_summary,
        revision_before: head.revision,
    })
}

#[command]
pub async fn jvm_commit_patch(input: CommitRequest) -> Result<CommitResult, AppError> {
    // Validate patch ops quickly first.
    let diags = validate_patch_ops(&input.patch_ops);
    if diags.iter().any(|d| d.level == DiagnosticLevel::Error) {
        return Err(app_err_jvm(
            "E_JVM_SCHEMA_INVALID",
            "patch_ops invalid".to_string(),
            false,
            Some(serde_json::json!({"diagnostics": diags})),
        ));
    }

    commit_patch_with_occ(
        &input.project_path,
        &input.chapter_path,
        input.base_revision,
        &input.call_id,
        input.actor,
        input.patch_ops,
    )
}

/// Repair all chapters in a project by assigning UUIDs to blocks that lack IDs.
/// Returns the number of chapters repaired and total blocks fixed.
#[command]
pub async fn jvm_repair_block_ids(project_path: String) -> Result<serde_json::Value, AppError> {
    let manuscripts_root = PathBuf::from(&project_path).join(MANUSCRIPTS_DIR);
    if !manuscripts_root.exists() {
        return Ok(serde_json::json!({
            "repaired_chapters": 0,
            "repaired_blocks": 0,
            "message": "manuscripts directory not found"
        }));
    }

    let volume_dirs = crate::services::list_dirs(&manuscripts_root).unwrap_or_default();
    let mut total_chapters = 0u32;
    let mut total_blocks = 0u32;

    for vol_dir in &volume_dirs {
        let vol_path = manuscripts_root.join(vol_dir);
        let chapter_files = list_files(&vol_path, ".json").unwrap_or_default();

        for ch_file in &chapter_files {
            if ch_file == "volume.json" {
                continue;
            }
            let ch_path = vol_path.join(ch_file);
            let mut chapter: Chapter = match read_json(&ch_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let repaired = ensure_doc_block_ids(&mut chapter.content);
            if repaired > 0 {
                chapter.updated_at = chrono::Utc::now().timestamp_millis();
                if let Err(e) = write_json(&ch_path, &chapter) {
                    tracing::warn!(
                        target: "jvm",
                        path = %ch_path.display(),
                        error = %e.message,
                        "failed to write repaired chapter"
                    );
                    continue;
                }
                total_chapters += 1;
                total_blocks += repaired as u32;
            }
        }
    }

    Ok(serde_json::json!({
        "repaired_chapters": total_chapters,
        "repaired_blocks": total_blocks,
        "message": if total_blocks == 0 {
            "all chapters already have valid block IDs".to_string()
        } else {
            format!("repaired {} blocks in {} chapters", total_blocks, total_chapters)
        }
    }))
}
