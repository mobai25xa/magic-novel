use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::models::{AppError, Chapter, VolumeMetadata};
use crate::services::{ensure_dir, list_dirs, list_files, read_json, write_json};
use crate::utils::atomic_write::atomic_write_json;

const MANUSCRIPTS_DIR: &str = "manuscripts";
const VOLUME_FILE: &str = "volume.json";

pub fn migrate_manuscripts_to_id_layout(project_path: &Path) -> Result<(), AppError> {
    let manuscripts_dir = project_path.join(MANUSCRIPTS_DIR);
    if !manuscripts_dir.exists() {
        return Ok(());
    }

    ensure_dir(&manuscripts_dir)?;

    let dirs = list_dirs(&manuscripts_dir)?;

    for dir_name in dirs {
        let current_volume_dir = manuscripts_dir.join(&dir_name);
        let volume_file = current_volume_dir.join(VOLUME_FILE);
        if !volume_file.exists() {
            continue;
        }

        let mut volume: VolumeMetadata = read_json(&volume_file)?;

        if volume.volume_id.trim().is_empty() {
            volume.volume_id = uuid::Uuid::new_v4().to_string();
            volume.updated_at = chrono::Utc::now().timestamp_millis();
            write_json(&volume_file, &volume)?;
        }

        let desired_volume_dir = manuscripts_dir.join(&volume.volume_id);
        if desired_volume_dir != current_volume_dir {
            if desired_volume_dir.exists() {
                return Err(AppError::invalid_argument("迁移失败：目标卷目录已存在"));
            }
            std::fs::rename(&current_volume_dir, &desired_volume_dir)?;
        }

        migrate_volume_chapters(&desired_volume_dir)?;
    }

    Ok(())
}

fn migrate_volume_chapters(volume_dir: &PathBuf) -> Result<(), AppError> {
    let volume_file = volume_dir.join(VOLUME_FILE);
    if !volume_file.exists() {
        return Ok(());
    }

    let mut volume: VolumeMetadata = read_json(&volume_file)?;

    let (id_by_old, ids_in_dir) = migrate_chapter_files(volume_dir)?;
    volume.chapter_order = rebuild_chapter_order(&volume.chapter_order, &id_by_old, &ids_in_dir);
    volume.updated_at = chrono::Utc::now().timestamp_millis();
    write_json(&volume_file, &volume)?;

    Ok(())
}

fn migrate_chapter_files(
    volume_dir: &PathBuf,
) -> Result<(HashMap<String, String>, Vec<String>), AppError> {
    let files = list_files(volume_dir, ".json")?;
    let mut id_by_old: HashMap<String, String> = HashMap::new();
    let mut ids_in_dir: Vec<String> = vec![];
    let mut seen_ids: HashSet<String> = HashSet::new();

    for file_name in files {
        if file_name == VOLUME_FILE {
            continue;
        }

        let current_path = volume_dir.join(&file_name);
        let mut chapter: Chapter = match read_json(&current_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let chapter_id = normalize_chapter_id(&mut chapter, &current_path, &seen_ids)?;
        seen_ids.insert(chapter_id.clone());
        ids_in_dir.push(chapter_id.clone());

        register_chapter_alias(&mut id_by_old, &file_name, &chapter_id);
        reconcile_chapter_file_name(
            volume_dir,
            &file_name,
            &current_path,
            &mut chapter,
            &chapter_id,
            &mut id_by_old,
        )?;
    }

    Ok((id_by_old, ids_in_dir))
}

fn normalize_chapter_id(
    chapter: &mut Chapter,
    current_path: &std::path::Path,
    seen_ids: &HashSet<String>,
) -> Result<String, AppError> {
    let mut chapter_id = chapter.id.clone();
    if chapter_id.trim().is_empty() || seen_ids.contains(&chapter_id) {
        chapter_id = uuid::Uuid::new_v4().to_string();
        chapter.id = chapter_id.clone();
        chapter.updated_at = chrono::Utc::now().timestamp_millis();
        atomic_write_json(current_path, chapter)?;
    }
    Ok(chapter_id)
}

fn register_chapter_alias(
    id_by_old: &mut HashMap<String, String>,
    file_name: &str,
    chapter_id: &str,
) {
    id_by_old.insert(file_name.to_string(), chapter_id.to_string());
    id_by_old.insert(
        file_name.trim_end_matches(".json").to_string(),
        chapter_id.to_string(),
    );
}

fn reconcile_chapter_file_name(
    volume_dir: &PathBuf,
    file_name: &str,
    current_path: &std::path::Path,
    chapter: &mut Chapter,
    chapter_id: &str,
    id_by_old: &mut HashMap<String, String>,
) -> Result<(), AppError> {
    let desired_name = format!("{}.json", chapter_id);
    let desired_path = volume_dir.join(&desired_name);

    if file_name == desired_name {
        return Ok(());
    }

    if desired_path.exists() {
        let new_id = uuid::Uuid::new_v4().to_string();
        chapter.id = new_id.clone();
        chapter.updated_at = chrono::Utc::now().timestamp_millis();
        atomic_write_json(current_path, chapter)?;
        let new_path = volume_dir.join(format!("{}.json", new_id));
        std::fs::rename(current_path, &new_path)?;
        register_chapter_alias(id_by_old, file_name, &new_id);
        return Ok(());
    }

    std::fs::rename(current_path, &desired_path)?;
    Ok(())
}

fn rebuild_chapter_order(
    previous_order: &[String],
    id_by_old: &HashMap<String, String>,
    ids_in_dir: &[String],
) -> Vec<String> {
    let mut new_order: Vec<String> = vec![];
    let mut added: HashSet<String> = HashSet::new();

    for entry in previous_order {
        if let Some(id) = id_by_old.get(entry) {
            if added.insert(id.clone()) {
                new_order.push(id.clone());
            }
            continue;
        }

        let with_ext = format!("{}.json", entry);
        if let Some(id) = id_by_old.get(&with_ext) {
            if added.insert(id.clone()) {
                new_order.push(id.clone());
            }
        }
    }

    for id in ids_in_dir {
        if added.insert(id.clone()) {
            new_order.push(id.clone());
        }
    }

    new_order
}
