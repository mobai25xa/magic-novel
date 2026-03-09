use std::path::PathBuf;

use serde::Serialize;
use tauri::command;

use crate::models::{AppError, AssetSource, AssetTree};
use crate::services::{ensure_dir, list_files, read_json};
use crate::utils::atomic_write::atomic_write_json;

use super::asset_support::{
    asset_dir, build_magic_assets_tree, dir_modified_at, ensure_safe_relative_path,
    write_folder_title, MagicAssetNode, MAGIC_ASSETS_DIR,
};

#[derive(Debug, Clone, Serialize)]
pub struct AssetSummary {
    pub id: String,
    pub title: String,
    pub modified_at: Option<i64>,
}

#[command]
pub async fn list_assets(
    project_path: String,
    kind: String,
) -> Result<Vec<AssetSummary>, AppError> {
    let project_path = PathBuf::from(&project_path);
    let dir = asset_dir(&project_path, &kind)?;

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut result = vec![];

    let files = list_files(&dir, ".json")?;
    for file_name in files {
        let full_path = dir.join(&file_name);
        let asset: AssetTree = match read_json(&full_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let modified_at = dir_modified_at(&full_path);

        result.push(AssetSummary {
            id: asset.id,
            title: asset.title,
            modified_at,
        });
    }

    Ok(result)
}

#[command]
pub async fn read_asset(
    project_path: String,
    kind: String,
    asset_id: String,
) -> Result<AssetTree, AppError> {
    let project_path = PathBuf::from(&project_path);
    let dir = asset_dir(&project_path, &kind)?;
    let path = dir.join(format!("{}.json", asset_id));

    read_json(&path)
}

#[command]
pub async fn save_asset(
    project_path: String,
    kind: String,
    asset: AssetTree,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let dir = asset_dir(&project_path, &kind)?;
    ensure_dir(&dir)?;

    let path = dir.join(format!("{}.json", asset.id));
    atomic_write_json(&path, &asset)?;

    Ok(())
}

#[command]
pub async fn copy_asset(
    from_project_path: String,
    to_project_path: String,
    kind: String,
    asset_id: String,
) -> Result<String, AppError> {
    let from_project_path = PathBuf::from(&from_project_path);
    let to_project_path = PathBuf::from(&to_project_path);

    let from_dir = asset_dir(&from_project_path, &kind)?;
    let from_path = from_dir.join(format!("{}.json", asset_id));

    let mut asset: AssetTree = read_json(&from_path)?;

    let new_id = uuid::Uuid::new_v4().to_string();
    asset.id = new_id.clone();

    let now = chrono::Utc::now().timestamp_millis();
    asset.source = Some(AssetSource {
        original_filename: asset.source.and_then(|s| s.original_filename),
        imported_at: now,
        importer: "copy".to_string(),
    });

    let to_dir = asset_dir(&to_project_path, &kind)?;
    ensure_dir(&to_dir)?;

    let to_path = to_dir.join(format!("{}.json", new_id));
    atomic_write_json(&to_path, &asset)?;

    Ok(new_id)
}

#[command]
pub async fn get_magic_assets_tree(project_path: String) -> Result<Vec<MagicAssetNode>, AppError> {
    let project_path = PathBuf::from(&project_path);
    let base_dir = project_path.join(MAGIC_ASSETS_DIR);

    build_magic_assets_tree(&base_dir, "")
}

#[command]
pub async fn read_magic_asset(
    project_path: String,
    relative_path: String,
) -> Result<AssetTree, AppError> {
    let project_path = PathBuf::from(&project_path);
    let rel = ensure_safe_relative_path(&relative_path)?;
    let full_path = project_path.join(MAGIC_ASSETS_DIR).join(rel);

    read_json(&full_path)
}

#[command]
pub async fn save_magic_asset(
    project_path: String,
    relative_path: String,
    asset: AssetTree,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let rel = ensure_safe_relative_path(&relative_path)?;

    let full_path = project_path.join(MAGIC_ASSETS_DIR).join(rel);
    if let Some(parent) = full_path.parent() {
        ensure_dir(parent)?;
    }

    atomic_write_json(&full_path, &asset)?;
    Ok(())
}

#[command]
pub async fn create_magic_asset_folder(
    project_path: String,
    parent_relative_dir: String,
    title: String,
) -> Result<String, AppError> {
    let project_path = PathBuf::from(&project_path);

    let parent_rel = ensure_safe_relative_path(&parent_relative_dir)?;
    let folder_id = uuid::Uuid::new_v4().to_string();
    let folder_rel = parent_rel.join(&folder_id);

    let full_path = project_path.join(MAGIC_ASSETS_DIR).join(&folder_rel);
    ensure_dir(&full_path)?;
    write_folder_title(&full_path, &title)?;

    Ok(folder_rel.to_string_lossy().replace('\\', "/"))
}

#[command]
pub async fn create_magic_asset_file(
    project_path: String,
    parent_relative_dir: String,
    asset_kind: String,
    title: String,
) -> Result<String, AppError> {
    let project_path = PathBuf::from(&project_path);
    let parent_rel = ensure_safe_relative_path(&parent_relative_dir)?;

    let id = uuid::Uuid::new_v4().to_string();
    let filename = format!("{}.json", id);
    let file_rel = parent_rel.join(&filename);

    let kind_enum = match asset_kind.as_str() {
        "lore" => crate::models::AssetKind::Lore,
        "prompt" => crate::models::AssetKind::Prompt,
        "worldview" => crate::models::AssetKind::Worldview,
        "outline" => crate::models::AssetKind::Outline,
        "character" => crate::models::AssetKind::Character,
        _ => return Err(AppError::invalid_argument("无效的资产类型")),
    };

    let now = chrono::Utc::now().timestamp_millis();
    let empty_root = crate::models::AssetNode {
        node_id: uuid::Uuid::new_v4().to_string(),
        title: "root".to_string(),
        level: 0,
        content: String::new(),
        children: vec![],
        tags: None,
    };

    let asset = AssetTree {
        schema_version: 1,
        id: id.clone(),
        kind: kind_enum,
        title,
        source: Some(AssetSource {
            original_filename: None,
            imported_at: now,
            importer: "create".to_string(),
        }),
        root: empty_root,
    };

    let full_path = project_path.join(MAGIC_ASSETS_DIR).join(&file_rel);
    if let Some(parent) = full_path.parent() {
        ensure_dir(parent)?;
    }
    atomic_write_json(&full_path, &asset)?;

    Ok(file_rel.to_string_lossy().replace('\\', "/"))
}

#[command]
pub async fn update_magic_asset_title(
    project_path: String,
    relative_path: String,
    new_title: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let rel = ensure_safe_relative_path(&relative_path)?;
    let full_path = project_path.join(MAGIC_ASSETS_DIR).join(rel);

    let mut asset: AssetTree = read_json(&full_path)?;
    asset.title = new_title;
    atomic_write_json(&full_path, &asset)?;

    Ok(())
}

#[command]
pub async fn update_magic_asset_folder_title(
    project_path: String,
    relative_dir: String,
    new_title: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let rel = ensure_safe_relative_path(&relative_dir)?;
    let full_path = project_path.join(MAGIC_ASSETS_DIR).join(rel);

    write_folder_title(&full_path, &new_title)?;
    Ok(())
}

#[command]
pub async fn delete_magic_asset_path(
    project_path: String,
    relative_path: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let rel = ensure_safe_relative_path(&relative_path)?;
    let full_path = project_path.join(MAGIC_ASSETS_DIR).join(rel);

    if !full_path.exists() {
        return Ok(());
    }

    let meta = std::fs::metadata(&full_path)?;
    if meta.is_dir() {
        std::fs::remove_dir_all(&full_path)?;
    } else {
        std::fs::remove_file(&full_path)?;
    }

    Ok(())
}
