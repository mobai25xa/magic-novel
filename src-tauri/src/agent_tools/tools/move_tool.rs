use std::path::PathBuf;

use serde_json::json;

use crate::agent_tools::contracts::MoveInput;
use crate::application::command_usecases::chapter::move_chapter_usecase;
use crate::application::command_usecases::volume::read_volume_usecase;
use crate::models::AppError;

const MANUSCRIPTS_DIR: &str = "manuscripts";

pub fn run(input: MoveInput, _call_id: &str) -> Result<serde_json::Value, AppError> {
    validate(&input)?;

    let chapter_path = input.chapter_path.trim();
    let target_volume_path = input.target_volume_path.trim();
    let (source_volume_path, chapter_file, chapter_id) = split_chapter_path(chapter_path)?;

    let target_volume = read_volume_usecase(&input.project_path, target_volume_path)?;
    let mut effective_target_index =
        (input.target_index as usize).min(target_volume.chapter_order.len());

    let same_volume = source_volume_path == target_volume_path;
    let mut no_op = false;
    if same_volume {
        let source_volume = read_volume_usecase(&input.project_path, source_volume_path)?;
        let current_index = source_volume
            .chapter_order
            .iter()
            .position(|id| id == &chapter_id)
            .ok_or_else(|| {
                AppError::invalid_argument(
                    "E_TOOL_INVALID_STATE: chapter_id not found in source volume order",
                )
            })?;

        let mut reordered = source_volume.chapter_order;
        reordered.retain(|id| id != &chapter_id);
        effective_target_index = effective_target_index.min(reordered.len());
        no_op = current_index == effective_target_index;
    }

    let new_chapter_path = format!("{}/{}", target_volume_path, chapter_file);

    if input.dry_run {
        return Ok(json!({
            "mode": "preview",
            "accepted": true,
            "chapter_path": chapter_path,
            "target_volume_path": target_volume_path,
            "target_index": effective_target_index,
            "new_chapter_path": new_chapter_path,
            "same_volume": same_volume,
            "no_op": no_op,
        }));
    }

    if no_op {
        return Ok(json!({
            "mode": "commit",
            "accepted": true,
            "chapter_path": chapter_path,
            "target_volume_path": target_volume_path,
            "target_index": effective_target_index,
            "new_chapter_path": chapter_path,
            "same_volume": true,
            "no_op": true,
        }));
    }

    let committed_path = move_chapter_usecase(
        &input.project_path,
        chapter_path,
        target_volume_path,
        effective_target_index as i32,
    )?;

    Ok(json!({
        "mode": "commit",
        "accepted": true,
        "chapter_path": chapter_path,
        "target_volume_path": target_volume_path,
        "target_index": effective_target_index,
        "new_chapter_path": committed_path,
        "same_volume": same_volume,
        "no_op": false,
    }))
}

fn split_chapter_path(chapter_path: &str) -> Result<(&str, &str, String), AppError> {
    let (source_volume_path, chapter_file) = chapter_path
        .rsplit_once('/')
        .ok_or_else(|| AppError::invalid_argument("E_TOOL_SCHEMA_INVALID: invalid chapter_path"))?;

    let chapter_id = chapter_file.trim_end_matches(".json").to_string();
    if chapter_id.is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: invalid chapter filename",
        ));
    }

    Ok((source_volume_path, chapter_file, chapter_id))
}

fn validate(input: &MoveInput) -> Result<(), AppError> {
    if input.project_path.trim().is_empty()
        || input.chapter_path.trim().is_empty()
        || input.target_volume_path.trim().is_empty()
    {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: project_path/chapter_path/target_volume_path are required",
        ));
    }

    if input.target_index < 0 {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: target_index must be >= 0",
        ));
    }

    let chapter_full_path = PathBuf::from(&input.project_path)
        .join(MANUSCRIPTS_DIR)
        .join(input.chapter_path.trim());
    if !chapter_full_path.exists() {
        return Err(AppError::not_found(
            "E_TOOL_NOT_FOUND: chapter path not found",
        ));
    }

    let target_volume_dir = PathBuf::from(&input.project_path)
        .join(MANUSCRIPTS_DIR)
        .join(input.target_volume_path.trim());
    if !target_volume_dir.exists() {
        return Err(AppError::not_found(
            "E_TOOL_NOT_FOUND: target volume path not found",
        ));
    }

    let _ = read_volume_usecase(&input.project_path, input.target_volume_path.trim())?;
    Ok(())
}
