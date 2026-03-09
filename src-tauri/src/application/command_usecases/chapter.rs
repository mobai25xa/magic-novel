use crate::models::{AppError, Chapter, VolumeMetadata};
use crate::services::{count_text, read_json, write_json};
use crate::utils::atomic_write::atomic_write_json;
use std::path::PathBuf;
use tauri::command;

const MANUSCRIPTS_DIR: &str = "manuscripts";
const VOLUME_FILE: &str = "volume.json";

fn manuscripts_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(MANUSCRIPTS_DIR)
}

fn chapter_full_path(project_path: &str, chapter_path: &str) -> PathBuf {
    manuscripts_dir(project_path).join(chapter_path)
}

pub fn create_chapter_usecase(
    project_path: &str,
    volume_path: &str,
    title: &str,
) -> Result<Chapter, AppError> {
    let chapter = Chapter::new(title.to_string());
    let chapter_filename = format!("{}.json", chapter.id);

    let volume_dir = manuscripts_dir(project_path).join(volume_path);
    let full_path = volume_dir.join(&chapter_filename);

    write_json(&full_path, &chapter)?;

    let volume_file_path = volume_dir.join(VOLUME_FILE);
    if volume_file_path.exists() {
        let mut volume: VolumeMetadata = read_json(&volume_file_path)?;
        volume.chapter_order.push(chapter.id.clone());
        volume.updated_at = chrono::Utc::now().timestamp_millis();
        write_json(&volume_file_path, &volume)?;
    }

    Ok(chapter)
}

pub fn read_chapter_usecase(project_path: &str, chapter_path: &str) -> Result<Chapter, AppError> {
    read_json(&chapter_full_path(project_path, chapter_path))
}

pub fn update_chapter_metadata_usecase(
    project_path: &str,
    chapter_path: &str,
    title: Option<String>,
    summary: Option<String>,
    status: Option<String>,
    target_words: Option<i32>,
    tags: Option<Vec<String>>,
    pinned_assets: Option<Vec<crate::models::ChapterAssetRef>>,
) -> Result<Chapter, AppError> {
    let full_path = chapter_full_path(project_path, chapter_path);
    let mut chapter: Chapter = read_json(&full_path)?;

    if let Some(t) = title {
        chapter.title = t;
    }
    if summary.is_some() {
        chapter.summary = summary;
    }
    if let Some(s) = status {
        chapter.status = match s.as_str() {
            "draft" => Some(crate::models::ChapterStatus::Draft),
            "revised" => Some(crate::models::ChapterStatus::Revised),
            "final" => Some(crate::models::ChapterStatus::Final),
            _ => chapter.status,
        };
    }
    if target_words.is_some() {
        chapter.target_words = target_words;
    }
    if tags.is_some() {
        chapter.tags = tags;
    }
    if pinned_assets.is_some() {
        chapter.pinned_assets = pinned_assets;
    }

    chapter.updated_at = chrono::Utc::now().timestamp_millis();
    write_json(&full_path, &chapter)?;

    Ok(chapter)
}

pub fn move_chapter_usecase(
    project_path: &str,
    chapter_path: &str,
    target_volume_path: &str,
    target_index: i32,
) -> Result<String, AppError> {
    let manuscripts_dir = manuscripts_dir(project_path);

    let path_parts: Vec<&str> = chapter_path.split('/').collect();
    if path_parts.len() < 2 {
        return Err(AppError::invalid_argument("Invalid chapter path"));
    }
    let source_volume_path = path_parts[0..path_parts.len() - 1].join("/");
    let chapter_filename = path_parts[path_parts.len() - 1].to_string();
    let chapter_id = chapter_filename.trim_end_matches(".json").to_string();

    let source_file = manuscripts_dir.join(chapter_path);
    let target_file = manuscripts_dir
        .join(target_volume_path)
        .join(&chapter_filename);
    let new_chapter_path = format!("{}/{}", target_volume_path, chapter_filename);

    let same_volume = source_volume_path == target_volume_path;

    if same_volume {
        let volume_file_path = manuscripts_dir.join(&source_volume_path).join(VOLUME_FILE);
        if volume_file_path.exists() {
            let mut volume: VolumeMetadata = read_json(&volume_file_path)?;
            volume.chapter_order.retain(|id| id != &chapter_id);
            let idx = (target_index as usize).min(volume.chapter_order.len());
            volume.chapter_order.insert(idx, chapter_id);
            volume.updated_at = chrono::Utc::now().timestamp_millis();
            write_json(&volume_file_path, &volume)?;
        }
    } else {
        if source_file.exists() {
            std::fs::rename(&source_file, &target_file)?;
        }

        let source_volume_file = manuscripts_dir.join(&source_volume_path).join(VOLUME_FILE);
        if source_volume_file.exists() {
            let mut volume: VolumeMetadata = read_json(&source_volume_file)?;
            volume.chapter_order.retain(|id| id != &chapter_id);
            volume.updated_at = chrono::Utc::now().timestamp_millis();
            write_json(&source_volume_file, &volume)?;
        }

        let target_volume_file = manuscripts_dir.join(target_volume_path).join(VOLUME_FILE);
        if target_volume_file.exists() {
            let mut volume: VolumeMetadata = read_json(&target_volume_file)?;
            let idx = (target_index as usize).min(volume.chapter_order.len());
            volume.chapter_order.insert(idx, chapter_id);
            volume.updated_at = chrono::Utc::now().timestamp_millis();
            write_json(&target_volume_file, &volume)?;
        }
    }

    Ok(new_chapter_path)
}

#[command]
pub async fn create_chapter(
    project_path: String,
    volume_path: String,
    title: String,
) -> Result<Chapter, AppError> {
    create_chapter_usecase(&project_path, &volume_path, &title)
}

#[command]
pub async fn read_chapter(project_path: String, chapter_path: String) -> Result<Chapter, AppError> {
    read_chapter_usecase(&project_path, &chapter_path)
}

#[command]
pub async fn save_chapter(
    project_path: String,
    chapter_path: String,
    content: serde_json::Value,
    title: Option<String>,
) -> Result<Chapter, AppError> {
    let full_path = PathBuf::from(&project_path)
        .join(MANUSCRIPTS_DIR)
        .join(&chapter_path);

    let mut chapter: Chapter = read_json(&full_path)?;

    chapter.content = content.clone();
    chapter.counts = count_text(&content);
    chapter.updated_at = chrono::Utc::now().timestamp_millis();

    if let Some(t) = title {
        chapter.title = t;
    }

    atomic_write_json(&full_path, &chapter)?;

    Ok(chapter)
}

#[command]
pub async fn update_chapter_metadata(
    project_path: String,
    chapter_path: String,
    title: Option<String>,
    summary: Option<String>,
    status: Option<String>,
    target_words: Option<i32>,
    tags: Option<Vec<String>>,
    pinned_assets: Option<Vec<crate::models::ChapterAssetRef>>,
) -> Result<Chapter, AppError> {
    update_chapter_metadata_usecase(
        &project_path,
        &chapter_path,
        title,
        summary,
        status,
        target_words,
        tags,
        pinned_assets,
    )
}

#[command]
pub async fn set_chapter_word_goal(
    project_path: String,
    chapter_path: String,
    word_goal: Option<i32>,
) -> Result<Chapter, AppError> {
    let full_path = PathBuf::from(&project_path)
        .join(MANUSCRIPTS_DIR)
        .join(&chapter_path);

    let mut chapter: Chapter = read_json(&full_path)?;
    chapter.target_words = word_goal;
    chapter.updated_at = chrono::Utc::now().timestamp_millis();

    atomic_write_json(&full_path, &chapter)?;

    Ok(chapter)
}

#[command]
pub async fn move_chapter(
    project_path: String,
    chapter_path: String,
    target_volume_path: String,
    target_index: i32,
) -> Result<String, AppError> {
    move_chapter_usecase(
        &project_path,
        &chapter_path,
        &target_volume_path,
        target_index,
    )
}

#[command]
pub async fn save_chapter_markdown(
    project_path: String,
    markdown_path: String,
    content: String,
) -> Result<(), AppError> {
    let full_path = PathBuf::from(&project_path)
        .join(MANUSCRIPTS_DIR)
        .join(&markdown_path);

    std::fs::write(&full_path, content)?;
    Ok(())
}
