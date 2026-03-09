use std::path::PathBuf;

use serde_json::json;

use crate::agent_tools::contracts::{DeleteInput, DeleteKind};
use crate::application::command_usecases::chapter::read_chapter_usecase;
use crate::application::command_usecases::recycle::{trash_chapter_usecase, trash_volume_usecase};
use crate::application::command_usecases::volume::read_volume_usecase;
use crate::models::AppError;

const MANUSCRIPTS_DIR: &str = "manuscripts";

pub fn run(input: DeleteInput, _call_id: &str) -> Result<serde_json::Value, AppError> {
    if input.project_path.trim().is_empty() || input.path.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: project_path/path are required",
        ));
    }

    match input.kind {
        DeleteKind::Chapter => delete_chapter(input),
        DeleteKind::Volume => delete_volume(input),
    }
}

fn delete_chapter(input: DeleteInput) -> Result<serde_json::Value, AppError> {
    let chapter_path = input.path.trim();
    let chapter_full_path = PathBuf::from(&input.project_path)
        .join(MANUSCRIPTS_DIR)
        .join(chapter_path);
    if !chapter_full_path.exists() {
        return Err(AppError::not_found(
            "E_TOOL_NOT_FOUND: chapter path not found",
        ));
    }

    let chapter = read_chapter_usecase(&input.project_path, chapter_path)?;
    let volume_path = chapter_path
        .rsplit_once('/')
        .map(|(volume, _)| volume.to_string())
        .unwrap_or_default();

    let impact = json!({
        "chapter_id": chapter.id,
        "title": chapter.title,
        "volume_path": volume_path,
    });

    if input.dry_run {
        return Ok(json!({
            "mode": "preview",
            "accepted": true,
            "kind": "chapter",
            "path": chapter_path,
            "impact": impact,
            "recycle": true,
        }));
    }

    trash_chapter_usecase(&input.project_path, chapter_path)?;

    Ok(json!({
        "mode": "commit",
        "accepted": true,
        "kind": "chapter",
        "path": chapter_path,
        "impact": impact,
        "recycle": true,
    }))
}

fn delete_volume(input: DeleteInput) -> Result<serde_json::Value, AppError> {
    let volume_path = input.path.trim();
    let volume_dir = PathBuf::from(&input.project_path)
        .join(MANUSCRIPTS_DIR)
        .join(volume_path);
    if !volume_dir.exists() {
        return Err(AppError::not_found(
            "E_TOOL_NOT_FOUND: volume path not found",
        ));
    }

    let volume = read_volume_usecase(&input.project_path, volume_path)?;
    let impact = json!({
        "chapter_count": volume.chapter_order.len(),
        "volume_id": volume.volume_id,
        "volume_title": volume.title,
    });

    if input.dry_run {
        return Ok(json!({
            "mode": "preview",
            "accepted": true,
            "kind": "volume",
            "path": volume_path,
            "impact": impact,
            "recycle": true,
        }));
    }

    trash_volume_usecase(&input.project_path, volume_path)?;

    Ok(json!({
        "mode": "commit",
        "accepted": true,
        "kind": "volume",
        "path": volume_path,
        "impact": impact,
        "recycle": true,
    }))
}
