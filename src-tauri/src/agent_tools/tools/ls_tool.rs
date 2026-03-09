use std::path::PathBuf;

use serde_json::json;

use crate::agent_tools::contracts::LsInput;
use crate::models::AppError;

pub fn run(input: LsInput, _call_id: &str) -> Result<serde_json::Value, AppError> {
    if input.project_path.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: project_path is required",
        ));
    }

    let cwd = normalize_cwd(&input.cwd);

    if cwd == "." {
        return ls_root(&input.project_path);
    }

    if cwd == ".magic_novel" {
        return ls_magic_root(&input.project_path);
    }

    if cwd.starts_with(".magic_novel/") {
        let rel = cwd.trim_start_matches(".magic_novel/");
        return ls_magic_subdir(&input.project_path, rel);
    }

    ls_volume(&input.project_path, &cwd)
}

fn normalize_cwd(cwd: &str) -> String {
    let c = cwd.trim().replace('\\', "/");
    if c.is_empty() {
        ".".to_string()
    } else {
        c.trim_end_matches('/').to_string()
    }
}

fn ls_root(project_path: &str) -> Result<serde_json::Value, AppError> {
    let manuscripts_root = PathBuf::from(project_path).join("manuscripts");
    let mut items: Vec<serde_json::Value> = vec![];

    if manuscripts_root.exists() {
        for entry in std::fs::read_dir(&manuscripts_root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let volume_dir = entry.path();
            let volume_file = volume_dir.join("volume.json");
            if !volume_file.exists() {
                continue;
            }

            let volume: crate::models::VolumeMetadata = crate::services::read_json(&volume_file)?;
            items.push(json!({
                "kind": "folder",
                "name": volume.title,
                "path": volume.volume_id,
                "child_count": volume.chapter_order.len(),
                "revision": 0,
                "metadata": {
                    "volume_id": volume.volume_id,
                    "type": "volume"
                }
            }));
        }
    }

    let magic_root = PathBuf::from(project_path).join("magic_assets");
    if magic_root.exists() {
        items.push(json!({
            "kind": "folder",
            "name": ".magic_novel",
            "path": ".magic_novel",
            "child_count": count_children(&magic_root),
            "revision": 0,
            "metadata": {
                "type": "knowledge_root"
            }
        }));
    }

    Ok(json!({
        "cwd": ".",
        "items": items,
    }))
}

fn ls_volume(project_path: &str, volume_path: &str) -> Result<serde_json::Value, AppError> {
    let volume_dir = PathBuf::from(project_path)
        .join("manuscripts")
        .join(volume_path);
    let volume_file = volume_dir.join("volume.json");

    if !volume_file.exists() {
        return Ok(json!({
            "cwd": volume_path,
            "items": []
        }));
    }

    let volume: crate::models::VolumeMetadata = crate::services::read_json(&volume_file)?;
    let mut items: Vec<serde_json::Value> = vec![];

    for chapter_id in &volume.chapter_order {
        let chapter_file_name = format!("{}.json", chapter_id);
        let chapter_file = volume_dir.join(&chapter_file_name);
        if !chapter_file.exists() {
            continue;
        }

        let chapter: crate::models::Chapter = crate::services::read_json(&chapter_file)?;
        items.push(json!({
            "kind": "domain_object",
            "name": chapter.title,
            "path": format!("{}/{}", volume_path, chapter_file_name),
            "child_count": 0,
            "revision": 0,
            "metadata": {
                "chapter_id": chapter.id,
                "type": "chapter",
                "status": chapter.status.map(|s| format!("{:?}", s).to_lowercase())
            }
        }));
    }

    Ok(json!({
        "cwd": volume_path,
        "items": items,
    }))
}

fn ls_magic_root(project_path: &str) -> Result<serde_json::Value, AppError> {
    let magic_root = PathBuf::from(project_path).join("magic_assets");
    if !magic_root.exists() {
        return Ok(json!({"cwd": ".magic_novel", "items": []}));
    }

    let mut items: Vec<serde_json::Value> = vec![];
    for entry in std::fs::read_dir(&magic_root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if entry.file_type()?.is_dir() {
            items.push(json!({
                "kind": "folder",
                "name": name,
                "path": format!(".magic_novel/{}", path.file_name().unwrap_or_default().to_string_lossy()),
                "child_count": count_children(&path),
                "revision": 0,
                "metadata": {"type": "knowledge_folder"}
            }));
        }
    }

    Ok(json!({
        "cwd": ".magic_novel",
        "items": items,
    }))
}

fn ls_magic_subdir(project_path: &str, rel: &str) -> Result<serde_json::Value, AppError> {
    let full = PathBuf::from(project_path).join("magic_assets").join(rel);

    if !full.exists() || !full.is_dir() {
        return Ok(json!({
            "cwd": format!(".magic_novel/{}", rel),
            "items": []
        }));
    }

    let mut items: Vec<serde_json::Value> = vec![];
    for entry in std::fs::read_dir(&full)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let child = entry.path();

        if entry.file_type()?.is_dir() {
            items.push(json!({
                "kind": "folder",
                "name": name,
                "path": format!(".magic_novel/{}/{}", rel, child.file_name().unwrap_or_default().to_string_lossy()),
                "child_count": count_children(&child),
                "revision": 0,
                "metadata": {"type": "knowledge_folder"}
            }));
        } else if name.ends_with(".json") {
            items.push(json!({
                "kind": "file",
                "name": name,
                "path": format!(".magic_novel/{}/{}", rel, name),
                "child_count": 0,
                "revision": 0,
                "metadata": {"type": "knowledge_file"}
            }));
        }
    }

    Ok(json!({
        "cwd": format!(".magic_novel/{}", rel),
        "items": items,
    }))
}

fn count_children(path: &PathBuf) -> usize {
    std::fs::read_dir(path)
        .ok()
        .map(|iter| iter.filter_map(Result::ok).count())
        .unwrap_or(0)
}
