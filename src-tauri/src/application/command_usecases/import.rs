use crate::models::{AppError, Chapter, VolumeMetadata};
use crate::services::ensure_dir;
use crate::services::{read_json, write_json};
use crate::utils::atomic_write::atomic_write_json;
use std::path::PathBuf;
use tauri::command;

use super::import_support::{
    build_chapter_content_json, ensure_volume_dir_with_meta, parse_manuscript_to_chapters,
    parse_to_asset_tree, read_supported_text,
};

#[command]
pub async fn import_asset(
    project_path: String,
    input_path: String,
    kind: String,
) -> Result<String, AppError> {
    let project_path = PathBuf::from(&project_path);
    let input_path = PathBuf::from(&input_path);
    let (content, extension) = read_supported_text(&input_path)?;

    let filename = input_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("未命名")
        .to_string();

    let asset_tree = parse_to_asset_tree(&content, &filename, &kind, &extension)?;

    let asset_dir = match kind.as_str() {
        "lore" => project_path.join("assets").join("lore"),
        "prompt" => project_path.join("assets").join("prompt"),
        "worldview" => project_path.join("assets").join("worldview"),
        "outline" => project_path.join("assets").join("outline"),
        "character" => project_path.join("assets").join("character"),
        _ => return Err(AppError::invalid_argument("无效的资产类型")),
    };

    ensure_dir(&asset_dir)?;
    let asset_file = asset_dir.join(format!("{}.json", asset_tree.id));
    atomic_write_json(&asset_file, &asset_tree)?;

    Ok(asset_tree.id.clone())
}

#[command]
pub async fn import_manuscript(project_path: String, input_path: String) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let input_path = PathBuf::from(&input_path);
    let (content, _) = read_supported_text(&input_path)?;

    let manuscripts_dir = project_path.join("manuscripts");
    ensure_dir(&manuscripts_dir)?;

    parse_manuscript_to_chapters(&content, &manuscripts_dir)?;

    Ok(())
}

#[command]
pub async fn import_manuscript_into_volume(
    project_path: String,
    volume_path: String,
    input_path: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let input_path = PathBuf::from(&input_path);
    let (content, _) = read_supported_text(&input_path)?;

    let manuscripts_dir = project_path.join("manuscripts");
    let volume_dir = ensure_volume_dir_with_meta(&manuscripts_dir, &volume_path)?;

    parse_manuscript_to_chapters(&content, &volume_dir)?;

    Ok(())
}

#[command]
pub async fn import_chapter(
    project_path: String,
    volume_path: String,
    input_path: String,
    title: Option<String>,
) -> Result<String, AppError> {
    let project_path = PathBuf::from(&project_path);
    let input_path = PathBuf::from(&input_path);
    let (content, _) = read_supported_text(&input_path)?;

    let default_title = input_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("未命名")
        .to_string();

    let chapter_title = title.unwrap_or(default_title);

    let manuscripts_dir = project_path.join("manuscripts");
    let volume_dir = ensure_volume_dir_with_meta(&manuscripts_dir, &volume_path)?;

    // Create a single chapter file from plain text
    let mut chapter = Chapter::new(chapter_title.clone());
    chapter.content = build_chapter_content_json(&content);

    let text_no_whitespace: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    chapter.counts.text_length_no_whitespace = text_no_whitespace.len() as i32;
    chapter.updated_at = chrono::Utc::now().timestamp_millis();

    let filename = format!("{}.json", chapter.id);
    let full_path = volume_dir.join(&filename);
    atomic_write_json(&full_path, &chapter)?;

    // Update chapter order (store chapter_id)
    let volume_file = volume_dir.join("volume.json");
    if volume_file.exists() {
        let mut volume_meta: VolumeMetadata = read_json(&volume_file)?;
        if !volume_meta.chapter_order.contains(&chapter.id) {
            volume_meta.chapter_order.push(chapter.id.clone());
            volume_meta.updated_at = chrono::Utc::now().timestamp_millis();
            write_json(&volume_file, &volume_meta)?;
        }
    }

    Ok(format!("{}/{}", volume_path, filename))
}
