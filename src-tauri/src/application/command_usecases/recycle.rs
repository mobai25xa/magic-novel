use crate::models::{
    AppError, Chapter, ProjectMetadata, RecycleItem, RecycleItemType, VolumeMetadata,
};
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::command;

const PROJECT_FILE: &str = "project.json";
const VOLUME_FILE: &str = "volume.json";
const MANUSCRIPTS_DIR: &str = "manuscripts";
const MAGIC_NOVEL_DIR: &str = "magic_novel";
const RECYCLE_DIR: &str = "recycle";
const RECYCLE_ITEMS_DIR: &str = "items";
const RECYCLE_INDEX_FILE: &str = "index.json";
const RECYCLE_PROJECTS_DIR: &str = "recycle_projects";
const RECYCLE_SCHEMA_VERSION: i32 = 1;
const RETENTION_DAYS: i64 = 30;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RecycleEntryKind {
    Chapter,
    Volume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecycleEntry {
    id: String,
    kind: RecycleEntryKind,
    name: String,
    origin: String,
    description: String,
    deleted_at: i64,
    expire_at: i64,
    original_rel_path: String,
    storage_rel_path: String,
    original_volume_path: Option<String>,
    chapter_id: Option<String>,
    original_chapter_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecycleIndex {
    schema_version: i32,
    items: Vec<RecycleEntry>,
}

impl Default for RecycleIndex {
    fn default() -> Self {
        Self {
            schema_version: RECYCLE_SCHEMA_VERSION,
            items: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecycledProjectEntry {
    id: String,
    name: String,
    author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cover_image: Option<String>,
    deleted_at: i64,
    expire_at: i64,
    original_project_path: String,
    storage_rel_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecycledProjectIndex {
    schema_version: i32,
    items: Vec<RecycledProjectEntry>,
}

impl Default for RecycledProjectIndex {
    fn default() -> Self {
        Self {
            schema_version: RECYCLE_SCHEMA_VERSION,
            items: vec![],
        }
    }
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn retention_ms() -> i64 {
    RETENTION_DAYS * DAY_MS
}

fn calc_days_remaining(expire_at: i64) -> i32 {
    let remaining = (expire_at - now_ts()).max(0);
    if remaining == 0 {
        return 0;
    }
    ((remaining + DAY_MS - 1) / DAY_MS) as i32
}

fn recycle_root(project_path: &Path) -> PathBuf {
    project_path.join(MAGIC_NOVEL_DIR).join(RECYCLE_DIR)
}

fn recycle_index_path(project_path: &Path) -> PathBuf {
    recycle_root(project_path).join(RECYCLE_INDEX_FILE)
}

fn projects_recycle_root(root_dir: &Path) -> PathBuf {
    root_dir.join(MAGIC_NOVEL_DIR).join(RECYCLE_PROJECTS_DIR)
}

fn projects_recycle_index_path(root_dir: &Path) -> PathBuf {
    projects_recycle_root(root_dir).join(RECYCLE_INDEX_FILE)
}

fn load_recycle_index(path: &Path) -> Result<RecycleIndex, AppError> {
    if !path.exists() {
        return Ok(RecycleIndex::default());
    }
    read_json(path)
}

fn save_recycle_index(path: &Path, index: &RecycleIndex) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    atomic_write_json(path, index)
}

fn load_recycled_project_index(path: &Path) -> Result<RecycledProjectIndex, AppError> {
    if !path.exists() {
        return Ok(RecycledProjectIndex::default());
    }
    read_json(path)
}

fn save_recycled_project_index(path: &Path, index: &RecycledProjectIndex) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    atomic_write_json(path, index)
}

fn remove_path(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }

    Ok(())
}

fn purge_expired_recycle_items(
    project_path: &Path,
    index: &mut RecycleIndex,
) -> Result<bool, AppError> {
    let now = now_ts();
    let root = recycle_root(project_path);
    let before = index.items.len();

    index.items.retain(|entry| {
        if entry.expire_at > now {
            return true;
        }
        let storage = root.join(&entry.storage_rel_path);
        let _ = remove_path(&storage);
        false
    });

    Ok(index.items.len() != before)
}

fn purge_expired_recycled_projects(
    root_dir: &Path,
    index: &mut RecycledProjectIndex,
) -> Result<bool, AppError> {
    let now = now_ts();
    let root = projects_recycle_root(root_dir);
    let before = index.items.len();

    index.items.retain(|entry| {
        if entry.expire_at > now {
            return true;
        }
        let storage = root.join(&entry.storage_rel_path);
        let _ = remove_path(&storage);
        false
    });

    Ok(index.items.len() != before)
}

fn project_name(project_path: &Path) -> Result<String, AppError> {
    let project: ProjectMetadata = read_json(&project_path.join(PROJECT_FILE))?;
    Ok(project.name)
}

fn normalize_rel(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}

#[command]
pub async fn list_recycle_items(project_path: String) -> Result<Vec<RecycleItem>, AppError> {
    let project_path = PathBuf::from(&project_path);
    let index_path = recycle_index_path(&project_path);
    let mut index = load_recycle_index(&index_path)?;

    if purge_expired_recycle_items(&project_path, &mut index)? {
        save_recycle_index(&index_path, &index)?;
    }

    index.items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));

    Ok(index
        .items
        .iter()
        .map(|entry| RecycleItem {
            id: entry.id.clone(),
            item_type: match entry.kind {
                RecycleEntryKind::Chapter => RecycleItemType::Chapter,
                RecycleEntryKind::Volume => RecycleItemType::Volume,
            },
            name: entry.name.clone(),
            origin: entry.origin.clone(),
            description: entry.description.clone(),
            deleted_at: entry.deleted_at,
            days_remaining: calc_days_remaining(entry.expire_at),
        })
        .collect())
}

#[command]
pub async fn trash_chapter(project_path: String, chapter_path: String) -> Result<(), AppError> {
    trash_chapter_usecase(&project_path, &chapter_path)
}

#[command]
pub async fn trash_volume(project_path: String, volume_path: String) -> Result<(), AppError> {
    trash_volume_usecase(&project_path, &volume_path)
}

pub fn trash_chapter_usecase(project_path: &str, chapter_path: &str) -> Result<(), AppError> {
    let project_path = PathBuf::from(project_path);
    let chapter_path = normalize_rel(chapter_path);

    let chapter_full_path = project_path.join(MANUSCRIPTS_DIR).join(&chapter_path);
    if !chapter_full_path.exists() {
        return Ok(());
    }

    let chapter: Chapter = read_json(&chapter_full_path)?;
    let project_name = project_name(&project_path)?;

    let path_parts: Vec<&str> = chapter_path.split('/').collect();
    if path_parts.len() < 2 {
        return Err(AppError::invalid_argument("Invalid chapter path"));
    }

    let volume_path = path_parts[..path_parts.len() - 1].join("/");
    let volume_file_path = project_path
        .join(MANUSCRIPTS_DIR)
        .join(&volume_path)
        .join(VOLUME_FILE);

    let mut original_chapter_index: Option<usize> = None;
    let mut volume_title = volume_path.clone();

    if volume_file_path.exists() {
        let volume: VolumeMetadata = read_json(&volume_file_path)?;
        let chapter_id = chapter.id.clone();
        original_chapter_index = volume.chapter_order.iter().position(|id| id == &chapter_id);
        volume_title = volume.title;
    }

    let item_id = uuid::Uuid::new_v4().to_string();
    let storage_rel_path = format!("{RECYCLE_ITEMS_DIR}/{item_id}.json");
    let storage_path = recycle_root(&project_path).join(&storage_rel_path);

    if let Some(parent) = storage_path.parent() {
        ensure_dir(parent)?;
    }
    std::fs::rename(&chapter_full_path, &storage_path)?;

    if volume_file_path.exists() {
        let mut volume: VolumeMetadata = read_json(&volume_file_path)?;
        volume.chapter_order.retain(|id| id != &chapter.id);
        volume.updated_at = now_ts();
        atomic_write_json(&volume_file_path, &volume)?;
    }

    let index_path = recycle_index_path(&project_path);
    let mut index = load_recycle_index(&index_path)?;
    let _ = purge_expired_recycle_items(&project_path, &mut index)?;

    let now = now_ts();
    index.items.push(RecycleEntry {
        id: item_id,
        kind: RecycleEntryKind::Chapter,
        name: chapter.title,
        origin: project_name,
        description: format!("{volume_title} / {chapter_path}"),
        deleted_at: now,
        expire_at: now + retention_ms(),
        original_rel_path: format!("{MANUSCRIPTS_DIR}/{chapter_path}"),
        storage_rel_path,
        original_volume_path: Some(volume_path),
        chapter_id: Some(chapter.id),
        original_chapter_index,
    });

    save_recycle_index(&index_path, &index)
}

pub fn trash_volume_usecase(project_path: &str, volume_path: &str) -> Result<(), AppError> {
    let project_path = PathBuf::from(project_path);
    let volume_path = normalize_rel(volume_path);

    let volume_full_path = project_path.join(MANUSCRIPTS_DIR).join(&volume_path);
    if !volume_full_path.exists() {
        return Ok(());
    }

    let volume_meta_path = volume_full_path.join(VOLUME_FILE);
    let volume_meta: VolumeMetadata = read_json(&volume_meta_path)?;
    let project_name = project_name(&project_path)?;

    let item_id = uuid::Uuid::new_v4().to_string();
    let storage_rel_path = format!("{RECYCLE_ITEMS_DIR}/{item_id}");
    let storage_path = recycle_root(&project_path).join(&storage_rel_path);

    if let Some(parent) = storage_path.parent() {
        ensure_dir(parent)?;
    }
    std::fs::rename(&volume_full_path, &storage_path)?;

    let index_path = recycle_index_path(&project_path);
    let mut index = load_recycle_index(&index_path)?;
    let _ = purge_expired_recycle_items(&project_path, &mut index)?;

    let now = now_ts();
    index.items.push(RecycleEntry {
        id: item_id,
        kind: RecycleEntryKind::Volume,
        name: volume_meta.title,
        origin: project_name,
        description: format!(
            "{volume_path}（{} chapters）",
            volume_meta.chapter_order.len()
        ),
        deleted_at: now,
        expire_at: now + retention_ms(),
        original_rel_path: format!("{MANUSCRIPTS_DIR}/{volume_path}"),
        storage_rel_path,
        original_volume_path: None,
        chapter_id: None,
        original_chapter_index: None,
    });

    save_recycle_index(&index_path, &index)
}

#[command]
pub async fn restore_recycle_item(project_path: String, item_id: String) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let index_path = recycle_index_path(&project_path);
    let mut index = load_recycle_index(&index_path)?;

    let Some(pos) = index.items.iter().position(|entry| entry.id == item_id) else {
        return Err(AppError::not_found("Recycle item not found"));
    };

    let entry = index.items[pos].clone();
    let storage_path = recycle_root(&project_path).join(&entry.storage_rel_path);
    let target_path = project_path.join(&entry.original_rel_path);

    if target_path.exists() {
        return Err(AppError::invalid_argument("目标路径已存在，无法还原"));
    }

    if let Some(parent) = target_path.parent() {
        ensure_dir(parent)?;
    }

    std::fs::rename(&storage_path, &target_path)?;

    if matches!(entry.kind, RecycleEntryKind::Chapter) {
        if let (Some(volume_path), Some(chapter_id)) =
            (entry.original_volume_path, entry.chapter_id)
        {
            let volume_file = project_path
                .join(MANUSCRIPTS_DIR)
                .join(volume_path)
                .join(VOLUME_FILE);

            if volume_file.exists() {
                let mut volume: VolumeMetadata = read_json(&volume_file)?;
                if !volume.chapter_order.iter().any(|id| id == &chapter_id) {
                    let idx = entry
                        .original_chapter_index
                        .unwrap_or(volume.chapter_order.len())
                        .min(volume.chapter_order.len());
                    volume.chapter_order.insert(idx, chapter_id);
                    volume.updated_at = now_ts();
                    atomic_write_json(&volume_file, &volume)?;
                }
            }
        }
    }

    index.items.remove(pos);
    save_recycle_index(&index_path, &index)
}

#[command]
pub async fn permanently_delete_recycle_item(
    project_path: String,
    item_id: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let index_path = recycle_index_path(&project_path);
    let mut index = load_recycle_index(&index_path)?;

    let Some(pos) = index.items.iter().position(|entry| entry.id == item_id) else {
        return Ok(());
    };

    let entry = index.items.remove(pos);
    let storage_path = recycle_root(&project_path).join(&entry.storage_rel_path);
    remove_path(&storage_path)?;

    save_recycle_index(&index_path, &index)
}

#[command]
pub async fn empty_recycle_bin(project_path: String) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let index_path = recycle_index_path(&project_path);
    let mut index = load_recycle_index(&index_path)?;

    let root = recycle_root(&project_path);
    for entry in &index.items {
        let storage_path = root.join(&entry.storage_rel_path);
        remove_path(&storage_path)?;
    }

    index.items.clear();
    save_recycle_index(&index_path, &index)
}

#[command]
pub async fn list_recycled_projects(root_dir: String) -> Result<Vec<RecycleItem>, AppError> {
    let root_dir = PathBuf::from(&root_dir);
    let index_path = projects_recycle_index_path(&root_dir);
    let mut index = load_recycled_project_index(&index_path)?;

    if purge_expired_recycled_projects(&root_dir, &mut index)? {
        save_recycled_project_index(&index_path, &index)?;
    }

    index.items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));

    Ok(index
        .items
        .iter()
        .map(|entry| RecycleItem {
            id: entry.id.clone(),
            item_type: RecycleItemType::Novel,
            name: entry.name.clone(),
            origin: if entry.author.trim().is_empty() {
                "Workspace".to_string()
            } else {
                entry.author.clone()
            },
            description: entry.original_project_path.clone(),
            deleted_at: entry.deleted_at,
            days_remaining: calc_days_remaining(entry.expire_at),
        })
        .collect())
}

#[command]
pub async fn trash_project(project_path: String) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);

    if !project_path.exists() {
        return Ok(());
    }

    if !project_path.is_dir() {
        return Err(AppError::invalid_argument("目标路径不是目录"));
    }

    let project_file = project_path.join(PROJECT_FILE);
    if !project_file.exists() {
        return Err(AppError::invalid_argument(
            "目标目录不是有效的作品目录（缺少 project.json）",
        ));
    }

    let project_meta: ProjectMetadata = read_json(&project_file)?;
    let Some(root_dir) = project_path.parent() else {
        return Err(AppError::invalid_argument("无法定位作品根目录"));
    };

    let item_id = uuid::Uuid::new_v4().to_string();
    let storage_rel_path = format!("{RECYCLE_ITEMS_DIR}/{item_id}");
    let storage_path = projects_recycle_root(root_dir).join(&storage_rel_path);

    if let Some(parent) = storage_path.parent() {
        ensure_dir(parent)?;
    }

    std::fs::rename(&project_path, &storage_path)?;

    let index_path = projects_recycle_index_path(root_dir);
    let mut index = load_recycled_project_index(&index_path)?;
    let _ = purge_expired_recycled_projects(root_dir, &mut index)?;

    let now = now_ts();
    index.items.push(RecycledProjectEntry {
        id: item_id,
        name: project_meta.name,
        author: project_meta.author,
        cover_image: project_meta.cover_image,
        deleted_at: now,
        expire_at: now + retention_ms(),
        original_project_path: project_path.to_string_lossy().to_string(),
        storage_rel_path,
    });

    save_recycled_project_index(&index_path, &index)
}

#[command]
pub async fn restore_recycled_project(root_dir: String, item_id: String) -> Result<(), AppError> {
    let root_dir = PathBuf::from(&root_dir);
    let index_path = projects_recycle_index_path(&root_dir);
    let mut index = load_recycled_project_index(&index_path)?;

    let Some(pos) = index.items.iter().position(|entry| entry.id == item_id) else {
        return Err(AppError::not_found("Recycled project not found"));
    };

    let entry = index.items[pos].clone();
    let storage_path = projects_recycle_root(&root_dir).join(&entry.storage_rel_path);
    let target_path = PathBuf::from(&entry.original_project_path);

    if target_path.exists() {
        return Err(AppError::invalid_argument("目标路径已存在，无法还原作品"));
    }

    if let Some(parent) = target_path.parent() {
        ensure_dir(parent)?;
    }

    std::fs::rename(&storage_path, &target_path)?;

    index.items.remove(pos);
    save_recycled_project_index(&index_path, &index)
}

#[command]
pub async fn permanently_delete_recycled_project(
    root_dir: String,
    item_id: String,
) -> Result<(), AppError> {
    let root_dir = PathBuf::from(&root_dir);
    let index_path = projects_recycle_index_path(&root_dir);
    let mut index = load_recycled_project_index(&index_path)?;

    let Some(pos) = index.items.iter().position(|entry| entry.id == item_id) else {
        return Ok(());
    };

    let entry = index.items.remove(pos);
    let storage_path = projects_recycle_root(&root_dir).join(&entry.storage_rel_path);
    remove_path(&storage_path)?;

    save_recycled_project_index(&index_path, &index)
}

#[command]
pub async fn empty_recycled_projects(root_dir: String) -> Result<(), AppError> {
    let root_dir = PathBuf::from(&root_dir);
    let index_path = projects_recycle_index_path(&root_dir);
    let mut index = load_recycled_project_index(&index_path)?;

    let root = projects_recycle_root(&root_dir);
    for entry in &index.items {
        let storage_path = root.join(&entry.storage_rel_path);
        remove_path(&storage_path)?;
    }

    index.items.clear();
    save_recycled_project_index(&index_path, &index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Chapter, ProjectMetadata, VolumeMetadata};
    use crate::services::{ensure_dir, write_json};

    fn create_temp_root() -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("magic_recycle_test_{}", uuid::Uuid::new_v4()));
        ensure_dir(&root).expect("create temp root");
        root
    }

    fn create_project(root: &Path, name: &str) -> PathBuf {
        let project_path = root.join(name);
        let manuscripts = project_path.join(MANUSCRIPTS_DIR);
        ensure_dir(&manuscripts).expect("create manuscripts dir");

        let project = ProjectMetadata::new(name.to_string(), "tester".to_string(), None, None);
        write_json(&project_path.join(PROJECT_FILE), &project).expect("write project json");
        project_path
    }

    fn create_volume_with_chapter(project_path: &Path) -> (String, String) {
        let volume = VolumeMetadata::new("卷一".to_string());
        let volume_path = volume.volume_id.clone();
        let volume_dir = project_path.join(MANUSCRIPTS_DIR).join(&volume_path);
        ensure_dir(&volume_dir).expect("create volume dir");

        let chapter = Chapter::new("第一章".to_string());
        let chapter_path = format!("{volume_path}/{}.json", chapter.id);

        let mut volume_to_save = volume.clone();
        volume_to_save.chapter_order = vec![chapter.id.clone()];

        write_json(&volume_dir.join(VOLUME_FILE), &volume_to_save).expect("write volume");
        write_json(
            &project_path.join(MANUSCRIPTS_DIR).join(&chapter_path),
            &chapter,
        )
        .expect("write chapter");

        (volume_path, chapter_path)
    }

    #[tokio::test]
    async fn trash_and_restore_chapter_roundtrip() {
        let root = create_temp_root();
        let project_path = create_project(&root, "novel_a");
        let (_volume_path, chapter_path) = create_volume_with_chapter(&project_path);

        trash_chapter(
            project_path.to_string_lossy().to_string(),
            chapter_path.clone(),
        )
        .await
        .expect("trash chapter");

        let listed = list_recycle_items(project_path.to_string_lossy().to_string())
            .await
            .expect("list recycle items");
        assert_eq!(listed.len(), 1);
        assert!(matches!(listed[0].item_type, RecycleItemType::Chapter));

        restore_recycle_item(
            project_path.to_string_lossy().to_string(),
            listed[0].id.clone(),
        )
        .await
        .expect("restore recycle item");

        assert!(project_path
            .join(MANUSCRIPTS_DIR)
            .join(chapter_path)
            .exists());
    }

    #[tokio::test]
    async fn trash_and_restore_project_roundtrip() {
        let root = create_temp_root();
        let project_path = create_project(&root, "novel_b");

        trash_project(project_path.to_string_lossy().to_string())
            .await
            .expect("trash project");

        assert!(!project_path.exists());

        let listed = list_recycled_projects(root.to_string_lossy().to_string())
            .await
            .expect("list recycled projects");
        assert_eq!(listed.len(), 1);
        assert!(matches!(listed[0].item_type, RecycleItemType::Novel));

        restore_recycled_project(root.to_string_lossy().to_string(), listed[0].id.clone())
            .await
            .expect("restore project");

        assert!(project_path.exists());
    }
}
