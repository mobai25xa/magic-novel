use crate::models::{
    AppError, Chapter, FileNode, ProjectMetadata, ProjectSnapshot, VolumeMetadata,
};
use crate::services::{ensure_dir, list_dirs, list_files, read_json, write_json};
use std::path::PathBuf;
use tauri::command;

const PROJECT_FILE: &str = "project.json";
const VOLUME_FILE: &str = "volume.json";
const MANUSCRIPTS_DIR: &str = "manuscripts";

#[command]
pub async fn create_project(
    path: String,
    name: String,
    author: String,
    project_type: Option<Vec<String>>,
    cover_image: Option<String>,
) -> Result<ProjectSnapshot, AppError> {
    let project_path = PathBuf::from(&path);
    ensure_dir(&project_path)?;

    let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);
    ensure_dir(&manuscripts_path)?;

    let normalized_cover_image =
        cover_image.and_then(|ci| if ci.trim().is_empty() { None } else { Some(ci) });

    let project = ProjectMetadata::new(name, author, project_type, normalized_cover_image);
    let project_file = project_path.join(PROJECT_FILE);
    write_json(&project_file, &project)?;

    Ok(ProjectSnapshot {
        project,
        path: project_path.to_string_lossy().to_string(),
        tree: vec![],
    })
}

#[command]
pub async fn open_project(path: String) -> Result<ProjectSnapshot, AppError> {
    let project_path = PathBuf::from(&path);
    let project_file = project_path.join(PROJECT_FILE);

    let mut project: ProjectMetadata = read_json(&project_file)?;
    project.last_opened_at = Some(chrono::Utc::now().timestamp_millis());
    write_json(&project_file, &project)?;

    // Migrate old manuscripts layout (title-based paths) to ID-based layout.
    crate::services::migrate_manuscripts_to_id_layout(&project_path)?;

    let tree = build_file_tree(&project_path.join(MANUSCRIPTS_DIR), "")?;

    Ok(ProjectSnapshot {
        project,
        path: project_path.to_string_lossy().to_string(),
        tree,
    })
}

#[command]
pub async fn get_project_tree(path: String) -> Result<Vec<FileNode>, AppError> {
    let project_path = PathBuf::from(&path);
    build_file_tree(&project_path.join(MANUSCRIPTS_DIR), "")
}

#[command]
pub async fn scan_projects_directory(root_dir: String) -> Result<Vec<ProjectSnapshot>, AppError> {
    let root_path = PathBuf::from(&root_dir);

    if !root_path.exists() || !root_path.is_dir() {
        return Ok(vec![]);
    }

    let mut projects = vec![];

    // List all directories in the root directory
    if let Ok(dirs) = list_dirs(&root_path) {
        for dir_name in dirs {
            let project_path = root_path.join(&dir_name);
            let project_file = project_path.join(PROJECT_FILE);

            // Check if this directory contains a project.json file
            if project_file.exists() {
                match read_json::<ProjectMetadata>(&project_file) {
                    Ok(project) => {
                        // Try to build the tree, but don't fail if it doesn't work
                        let tree = build_file_tree(&project_path.join(MANUSCRIPTS_DIR), "")
                            .unwrap_or_default();
                        projects.push(ProjectSnapshot {
                            project,
                            path: project_path.to_string_lossy().to_string(),
                            tree,
                        });
                    }
                    Err(_) => {
                        // Skip directories that don't have valid project.json
                        continue;
                    }
                }
            }
        }
    }

    Ok(projects)
}

#[command]
pub async fn update_project_metadata(
    path: String,
    name: Option<String>,
    author: Option<String>,
    description: Option<String>,
    project_type: Option<Vec<String>>,
    cover_image: Option<String>,
) -> Result<ProjectMetadata, AppError> {
    let project_path = PathBuf::from(&path);
    let project_file = project_path.join(PROJECT_FILE);

    let mut project: ProjectMetadata = read_json(&project_file)?;

    if let Some(n) = name {
        project.name = n;
    }
    if let Some(a) = author {
        project.author = a;
    }
    if description.is_some() {
        project.description = description;
    }
    if let Some(pt) = project_type {
        let mut out: Vec<String> = Vec::new();
        for s in pt {
            let s = s.trim().to_string();
            if s.is_empty() {
                continue;
            }
            if !out.contains(&s) {
                out.push(s);
            }
        }
        project.project_type = out;
    }

    if let Some(ci) = cover_image {
        project.cover_image = if ci.trim().is_empty() { None } else { Some(ci) };
    }

    project.updated_at = chrono::Utc::now().timestamp_millis();
    write_json(&project_file, &project)?;

    Ok(project)
}

fn build_file_tree(base_path: &PathBuf, relative_path: &str) -> Result<Vec<FileNode>, AppError> {
    let mut nodes = vec![];

    let dirs = list_dirs(base_path)?;
    for dir_name in dirs {
        let dir_path = base_path.join(&dir_name);
        let volume_file = dir_path.join(VOLUME_FILE);

        let rel_path = if relative_path.is_empty() {
            dir_name.clone()
        } else {
            format!("{}/{}", relative_path, dir_name)
        };

        if volume_file.exists() {
            let volume: VolumeMetadata = read_json(&volume_file)?;
            let children = build_chapter_nodes(&dir_path, &rel_path, &volume)?;
            nodes.push(FileNode::Dir {
                name: volume.title,
                path: rel_path,
                children,
                created_at: volume.created_at,
                updated_at: volume.updated_at,
            });
        }
    }

    Ok(nodes)
}

fn build_chapter_nodes(
    volume_path: &PathBuf,
    relative_path: &str,
    volume: &VolumeMetadata,
) -> Result<Vec<FileNode>, AppError> {
    let mut nodes = vec![];

    let files = list_files(volume_path, ".json")?;
    let mut chapter_map: std::collections::HashMap<String, FileNode> =
        std::collections::HashMap::new();
    let mut unordered: Vec<(String, FileNode)> = vec![];

    for file_name in files {
        if file_name == VOLUME_FILE {
            continue;
        }

        let file_path = volume_path.join(&file_name);
        let chapter: Chapter = read_json(&file_path)?;

        let chapter_id = chapter.id.clone();
        let rel_path = format!("{}/{}", relative_path, file_name);

        let node = FileNode::Chapter {
            name: chapter_id.clone(),
            path: rel_path,
            chapter_id,
            title: chapter.title,
            text_length_no_whitespace: chapter
                .counts
                .word_count
                .unwrap_or(chapter.counts.text_length_no_whitespace),
            word_count: chapter.counts.word_count,
            status: chapter.status.map(|s| format!("{:?}", s).to_lowercase()),
            created_at: chapter.created_at,
            updated_at: chapter.updated_at,
        };

        let filename_id = file_name.trim_end_matches(".json");
        if volume.chapter_order.contains(&filename_id.to_string()) {
            chapter_map.insert(filename_id.to_string(), node);
        } else {
            unordered.push((filename_id.to_string(), node));
        }
    }

    for chapter_id in &volume.chapter_order {
        if let Some(node) = chapter_map.remove(chapter_id) {
            nodes.push(node);
        }
    }

    for (_, node) in unordered {
        nodes.push(node);
    }

    Ok(nodes)
}
