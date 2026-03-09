use crate::models::{AppError, VolumeMetadata};
use crate::services::{ensure_dir, read_json, write_json};
use std::path::PathBuf;
use tauri::command;

const VOLUME_FILE: &str = "volume.json";
const MANUSCRIPTS_DIR: &str = "manuscripts";

fn volume_file_path(project_path: &str, volume_path: &str) -> PathBuf {
    PathBuf::from(project_path)
        .join(MANUSCRIPTS_DIR)
        .join(volume_path)
        .join(VOLUME_FILE)
}

pub fn create_volume_usecase(project_path: &str, title: &str) -> Result<VolumeMetadata, AppError> {
    let project_path = PathBuf::from(project_path);
    let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);

    let volume = VolumeMetadata::new(title.to_string());
    let volume_path = manuscripts_path.join(&volume.volume_id);

    ensure_dir(&volume_path)?;
    write_json(&volume_path.join(VOLUME_FILE), &volume)?;

    Ok(volume)
}

pub fn read_volume_usecase(
    project_path: &str,
    volume_path: &str,
) -> Result<VolumeMetadata, AppError> {
    read_json(&volume_file_path(project_path, volume_path))
}

pub fn update_volume_usecase(
    project_path: &str,
    volume_path: &str,
    title: Option<String>,
    summary: Option<String>,
) -> Result<VolumeMetadata, AppError> {
    let full_path = volume_file_path(project_path, volume_path);
    let mut volume: VolumeMetadata = read_json(&full_path)?;

    if let Some(t) = title {
        volume.title = t;
    }
    if summary.is_some() {
        volume.summary = summary;
    }

    volume.updated_at = chrono::Utc::now().timestamp_millis();
    write_json(&full_path, &volume)?;

    Ok(volume)
}

#[command]
pub async fn create_volume(
    project_path: String,
    title: String,
) -> Result<VolumeMetadata, AppError> {
    create_volume_usecase(&project_path, &title)
}

#[command]
pub async fn read_volume(
    project_path: String,
    volume_path: String,
) -> Result<VolumeMetadata, AppError> {
    read_volume_usecase(&project_path, &volume_path)
}

#[command]
pub async fn update_volume(
    project_path: String,
    volume_path: String,
    title: Option<String>,
    summary: Option<String>,
) -> Result<VolumeMetadata, AppError> {
    update_volume_usecase(&project_path, &volume_path, title, summary)
}
