use serde_json::json;

use crate::agent_tools::contracts::{CreateInput, CreateKind, NodeKind};
use crate::application::command_usecases::chapter::create_chapter_usecase;
use crate::application::command_usecases::volume::create_volume_usecase;
use crate::models::AppError;

pub fn run(input: CreateInput, _call_id: &str) -> Result<serde_json::Value, AppError> {
    if input.project_path.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: project_path is required",
        ));
    }

    let kind = resolve_kind(&input);
    let title = resolve_title(&input)?;

    match kind {
        CreateKind::Volume => create_volume(&input, &title),
        CreateKind::Chapter => create_chapter(&input, &title),
    }
}

fn resolve_kind(input: &CreateInput) -> CreateKind {
    if let Some(kind) = &input.kind {
        return kind.clone();
    }

    if let Some(kind) = input
        .metadata
        .get("kind")
        .and_then(|value| value.as_str())
        .map(|value| value.trim())
    {
        if kind.eq_ignore_ascii_case("volume") {
            return CreateKind::Volume;
        }
        if kind.eq_ignore_ascii_case("chapter") {
            return CreateKind::Chapter;
        }
    }

    match input.node_kind {
        NodeKind::Folder => CreateKind::Volume,
        _ => CreateKind::Chapter,
    }
}

fn resolve_title(input: &CreateInput) -> Result<String, AppError> {
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            input
                .metadata
                .get("title")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .or_else(|| {
            let legacy = input.name.trim();
            (!legacy.is_empty()).then_some(legacy)
        })
        .ok_or_else(|| AppError::invalid_argument("E_TOOL_SCHEMA_INVALID: title is required"))?;

    Ok(title.to_string())
}

fn resolve_volume_path(input: &CreateInput) -> Option<String> {
    input
        .volume_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            input
                .metadata
                .get("volume_path")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .or_else(|| {
            let legacy = input.cwd.trim();
            (!legacy.is_empty() && legacy != ".").then_some(legacy.to_string())
        })
}

fn create_chapter(input: &CreateInput, title: &str) -> Result<serde_json::Value, AppError> {
    let volume_path = resolve_volume_path(input).ok_or_else(|| {
        AppError::invalid_argument("E_TOOL_SCHEMA_INVALID: volume_path is required for chapter")
    })?;

    let chapter = create_chapter_usecase(&input.project_path, &volume_path, title)?;

    Ok(json!({
        "created_kind": "chapter",
        "path": format!("{}/{}.json", volume_path, chapter.id),
        "id": chapter.id,
        "created_at": chapter.created_at,
    }))
}

fn create_volume(input: &CreateInput, title: &str) -> Result<serde_json::Value, AppError> {
    let volume = create_volume_usecase(&input.project_path, title)?;
    let volume_id = volume.volume_id.clone();

    Ok(json!({
        "created_kind": "volume",
        "path": volume_id,
        "id": volume.volume_id,
        "created_at": volume.created_at,
    }))
}
