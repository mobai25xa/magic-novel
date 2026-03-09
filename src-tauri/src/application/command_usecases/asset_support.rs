use std::path::PathBuf;

use serde::Serialize;
use std::path::Component;

use crate::models::{AppError, AssetTree};
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;

pub const MAGIC_ASSETS_DIR: &str = "magic_assets";
pub const MAGIC_FOLDER_META: &str = ".magic_folder.json";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum MagicAssetNode {
    #[serde(rename = "dir")]
    Dir {
        name: String,
        path: String,
        title: Option<String>,
        children: Vec<MagicAssetNode>,
    },
    #[serde(rename = "file")]
    File {
        name: String,
        path: String,
        title: Option<String>,
        asset_id: Option<String>,
        asset_kind: Option<String>,
        modified_at: Option<i64>,
    },
}

pub fn asset_dir(project_path: &PathBuf, kind: &str) -> Result<PathBuf, AppError> {
    Ok(match kind {
        "lore" => project_path.join(MAGIC_ASSETS_DIR).join("lore"),
        "prompt" => project_path.join(MAGIC_ASSETS_DIR).join("prompt"),
        "worldview" => project_path.join(MAGIC_ASSETS_DIR).join("worldview"),
        "outline" => project_path.join(MAGIC_ASSETS_DIR).join("outline"),
        "character" => project_path.join(MAGIC_ASSETS_DIR).join("character"),
        _ => return Err(AppError::invalid_argument("无效的资产类型")),
    })
}

pub fn ensure_safe_relative_path(path: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(path);

    if p.is_absolute() {
        return Err(AppError::invalid_argument("不允许绝对路径"));
    }

    for c in p.components() {
        match c {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::invalid_argument("不允许包含上级目录"));
            }
            _ => {}
        }
    }

    Ok(p)
}

fn system_time_to_millis(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}

pub fn dir_modified_at(path: &PathBuf) -> Option<i64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(system_time_to_millis)
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct FolderMeta {
    pub title: String,
}

pub fn read_folder_title(dir: &PathBuf) -> Option<String> {
    let meta_path = dir.join(MAGIC_FOLDER_META);
    let meta: FolderMeta = read_json(&meta_path).ok()?;
    Some(meta.title)
}

pub fn write_folder_title(dir: &PathBuf, title: &str) -> Result<(), AppError> {
    ensure_dir(dir)?;
    let meta_path = dir.join(MAGIC_FOLDER_META);
    atomic_write_json(
        &meta_path,
        &FolderMeta {
            title: title.to_string(),
        },
    )?;
    Ok(())
}

pub fn build_magic_assets_tree(
    base_dir: &PathBuf,
    relative: &str,
) -> Result<Vec<MagicAssetNode>, AppError> {
    if !base_dir.exists() {
        return Ok(vec![]);
    }

    let mut out: Vec<MagicAssetNode> = vec![];

    for entry in std::fs::read_dir(base_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name().to_str().unwrap_or("").to_string();
        let path = entry.path();
        let rel_path = rel_join(relative, &name);

        if file_type.is_dir() {
            push_dir_node(&mut out, name, rel_path, path)?;
            continue;
        }

        if file_type.is_file() {
            maybe_push_file_node(&mut out, &name, rel_path, path)?;
        }
    }

    sort_magic_nodes(&mut out);
    Ok(out)
}

fn rel_join(relative: &str, name: &str) -> String {
    if relative.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", relative, name)
    }
}

fn push_dir_node(
    out: &mut Vec<MagicAssetNode>,
    name: String,
    rel_path: String,
    path: PathBuf,
) -> Result<(), AppError> {
    let children = build_magic_assets_tree(&path, &rel_path)?;
    let title = read_folder_title(&path);
    out.push(MagicAssetNode::Dir {
        name,
        path: rel_path,
        title,
        children,
    });
    Ok(())
}

fn maybe_push_file_node(
    out: &mut Vec<MagicAssetNode>,
    name: &str,
    rel_path: String,
    path: PathBuf,
) -> Result<(), AppError> {
    if name == MAGIC_FOLDER_META || !name.ends_with(".json") {
        return Ok(());
    }

    let modified_at = dir_modified_at(&path);
    let parsed: Option<AssetTree> = read_json(&path).ok();
    let (title, asset_id, asset_kind) = asset_file_meta(parsed);

    out.push(MagicAssetNode::File {
        name: name.to_string(),
        path: rel_path,
        title,
        asset_id,
        asset_kind,
        modified_at,
    });
    Ok(())
}

fn asset_file_meta(parsed: Option<AssetTree>) -> (Option<String>, Option<String>, Option<String>) {
    match parsed {
        Some(a) => {
            let kind = format!("{:?}", a.kind).to_lowercase();
            (Some(a.title), Some(a.id), Some(kind))
        }
        None => (None, None, None),
    }
}

fn sort_magic_nodes(out: &mut [MagicAssetNode]) {
    out.sort_by(|a, b| {
        let a_is_dir = matches!(a, MagicAssetNode::Dir { .. });
        let b_is_dir = matches!(b, MagicAssetNode::Dir { .. });
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let an = match a {
                    MagicAssetNode::Dir { name, .. } => name,
                    MagicAssetNode::File { name, .. } => name,
                };
                let bn = match b {
                    MagicAssetNode::Dir { name, .. } => name,
                    MagicAssetNode::File { name, .. } => name,
                };
                an.cmp(bn)
            }
        }
    });
}
