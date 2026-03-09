use crate::models::{AppError, Chapter, VolumeMetadata};
use crate::services::{ensure_dir, list_dirs, list_files, read_json};
use std::fs;
use std::path::PathBuf;
use tauri::command;

const MANUSCRIPTS_DIR: &str = "manuscripts";
const VOLUME_FILE: &str = "volume.json";

#[command]
pub async fn export_chapter(
    project_path: String,
    chapter_path: String,
    output_path: String,
    format: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let output_path = PathBuf::from(&output_path);

    let full_chapter_path = project_path.join(MANUSCRIPTS_DIR).join(&chapter_path);

    if !full_chapter_path.exists() {
        return Err(AppError::not_found("章节不存在"));
    }

    let chapter: Chapter = read_json(&full_chapter_path)?;
    let chapter_text = extract_chapter_text(&chapter.content);

    let mut content = String::new();
    match format.as_str() {
        "md" => {
            content.push_str(&format!("# {}\n\n", chapter.title));
            content.push_str(&chapter_text);
            content.push_str("\n");
        }
        "txt" => {
            content.push_str(&format!("{}\n\n", chapter.title));
            content.push_str(&chapter_text);
            content.push_str("\n");
        }
        _ => {
            content.push_str(&format!("# {}\n\n", chapter.title));
            content.push_str(&chapter_text);
            content.push_str("\n");
        }
    }

    if let Some(parent) = output_path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(&output_path, content)?;

    Ok(())
}

#[command]
pub async fn export_volume(
    project_path: String,
    volume_path: String,
    output_path: String,
    format: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let output_path = PathBuf::from(&output_path);

    let volume_dir = project_path.join(MANUSCRIPTS_DIR).join(&volume_path);

    if !volume_dir.exists() {
        return Err(AppError::not_found("卷不存在"));
    }

    let volume_file = volume_dir.join(VOLUME_FILE);
    if !volume_file.exists() {
        return Err(AppError::not_found("卷元数据不存在"));
    }

    let volume: VolumeMetadata = read_json(&volume_file)?;

    let mut content = String::new();
    match format.as_str() {
        "md" => content.push_str(&format!("# {}\n\n", volume.title)),
        "txt" => content.push_str(&format!("【{}】\n\n", volume.title)),
        _ => content.push_str(&format!("# {}\n\n", volume.title)),
    }

    let chapter_files = list_files(&volume_dir, ".json")?;
    for chapter_file in chapter_files {
        if chapter_file == VOLUME_FILE {
            continue;
        }

        let chapter_path = volume_dir.join(&chapter_file);
        let chapter: Chapter = read_json(&chapter_path)?;
        let chapter_text = extract_chapter_text(&chapter.content);

        match format.as_str() {
            "md" => {
                content.push_str(&format!("## {}\n\n", chapter.title));
                content.push_str(&chapter_text);
                content.push_str("\n\n");
            }
            "txt" => {
                content.push_str(&format!("{}\n\n", chapter.title));
                content.push_str(&chapter_text);
                content.push_str("\n\n");
            }
            _ => {
                content.push_str(&format!("## {}\n\n", chapter.title));
                content.push_str(&chapter_text);
                content.push_str("\n\n");
            }
        }
    }

    if let Some(parent) = output_path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(&output_path, content)?;

    Ok(())
}

#[command]
pub async fn export_book_single(
    project_path: String,
    output_path: String,
    format: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let output_path = PathBuf::from(&output_path);
    let manuscripts_dir = project_path.join(MANUSCRIPTS_DIR);

    if !manuscripts_dir.exists() {
        return Err(AppError::not_found("项目中没有稿件"));
    }

    let mut content = String::new();
    let volumes = list_dirs(&manuscripts_dir)?;

    for volume_name in volumes {
        let volume_dir = manuscripts_dir.join(&volume_name);
        let volume_file = volume_dir.join(VOLUME_FILE);

        if volume_file.exists() {
            let volume: VolumeMetadata = read_json(&volume_file)?;

            match format.as_str() {
                "md" => {
                    content.push_str(&format!("# {}\n\n", volume.title));
                }
                "txt" => {
                    content.push_str(&format!("【{}】\n\n", volume.title));
                }
                _ => {
                    content.push_str(&format!("# {}\n\n", volume.title));
                }
            }

            let chapter_files = list_files(&volume_dir, ".json")?;
            for chapter_file in chapter_files {
                if chapter_file == VOLUME_FILE {
                    continue;
                }

                let chapter_path = volume_dir.join(&chapter_file);
                let chapter: Chapter = read_json(&chapter_path)?;
                let chapter_text = extract_chapter_text(&chapter.content);

                match format.as_str() {
                    "md" => {
                        content.push_str(&format!("## {}\n\n", chapter.title));
                        content.push_str(&chapter_text);
                        content.push_str("\n\n");
                    }
                    "txt" => {
                        content.push_str(&format!("{}\n\n", chapter.title));
                        content.push_str(&chapter_text);
                        content.push_str("\n\n");
                    }
                    _ => {
                        content.push_str(&format!("## {}\n\n", chapter.title));
                        content.push_str(&chapter_text);
                        content.push_str("\n\n");
                    }
                }
            }
        }
    }

    if let Some(parent) = output_path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(&output_path, content)?;

    Ok(())
}

#[command]
pub async fn export_tree_multi(
    project_path: String,
    output_dir: String,
    format: String,
) -> Result<i32, AppError> {
    let project_path = PathBuf::from(&project_path);
    let output_dir = PathBuf::from(&output_dir);
    let manuscripts_dir = project_path.join(MANUSCRIPTS_DIR);

    if !manuscripts_dir.exists() {
        return Err(AppError::not_found("项目中没有稿件"));
    }

    ensure_dir(&output_dir)?;

    let mut file_count = 0;
    let volumes = list_dirs(&manuscripts_dir)?;
    let extension = match format.as_str() {
        "md" => "md",
        "txt" => "txt",
        _ => "md",
    };

    for volume_name in volumes {
        let volume_dir = manuscripts_dir.join(&volume_name);
        let volume_file = volume_dir.join(VOLUME_FILE);

        if volume_file.exists() {
            let volume: VolumeMetadata = read_json(&volume_file)?;
            let output_volume_dir = output_dir.join(sanitize_filename(&volume.title));
            ensure_dir(&output_volume_dir)?;

            let chapter_files = list_files(&volume_dir, ".json")?;
            for chapter_file in chapter_files {
                if chapter_file == VOLUME_FILE {
                    continue;
                }

                let chapter_path = volume_dir.join(&chapter_file);
                let chapter: Chapter = read_json(&chapter_path)?;
                let chapter_text = extract_chapter_text(&chapter.content);

                let mut content = String::new();
                match format.as_str() {
                    "md" => {
                        content.push_str(&format!("# {}\n\n", chapter.title));
                        content.push_str(&chapter_text);
                    }
                    "txt" => {
                        content.push_str(&format!("{}\n\n", chapter.title));
                        content.push_str(&chapter_text);
                    }
                    _ => {
                        content.push_str(&format!("# {}\n\n", chapter.title));
                        content.push_str(&chapter_text);
                    }
                }

                let output_filename =
                    format!("{}.{}", sanitize_filename(&chapter.title), extension);
                let output_file = output_volume_dir.join(output_filename);
                fs::write(&output_file, content)?;
                file_count += 1;
            }
        }
    }

    Ok(file_count)
}

fn extract_chapter_text(content: &serde_json::Value) -> String {
    let mut texts = Vec::new();
    extract_text_recursive(content, &mut texts);
    texts.join("\n\n")
}

fn extract_text_recursive(value: &serde_json::Value, texts: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(obj) => {
            if obj.get("type").and_then(|t| t.as_str()) == Some("paragraph")
                || obj.get("type").and_then(|t| t.as_str()) == Some("heading")
            {
                let para_text = extract_node_text(value);
                if !para_text.is_empty() {
                    texts.push(para_text);
                }
            } else if let Some(content) = obj.get("content") {
                extract_text_recursive(content, texts);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_text_recursive(item, texts);
            }
        }
        _ => {}
    }
}

fn extract_node_text(node: &serde_json::Value) -> String {
    let mut result = String::new();

    if let Some(content) = node.get("content") {
        if let Some(arr) = content.as_array() {
            for child in arr {
                if let Some(text) = child.get("text").and_then(|t| t.as_str()) {
                    result.push_str(text);
                } else {
                    result.push_str(&extract_node_text(child));
                }
            }
        }
    }

    result
}

fn sanitize_filename(name: &str) -> String {
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let mut result: String = name
        .chars()
        .map(|c| if invalid_chars.contains(&c) { '_' } else { c })
        .collect();

    result = result.trim().to_string();

    if result.is_empty() {
        result = "untitled".to_string();
    }

    if result.len() > 50 {
        result = result.chars().take(50).collect();
    }

    result
}
