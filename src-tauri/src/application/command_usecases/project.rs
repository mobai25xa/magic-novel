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
    description: Option<String>,
    target_total_words: Option<i32>,
    planned_volumes: Option<i32>,
    target_words_per_volume: Option<i32>,
    target_words_per_chapter: Option<i32>,
    narrative_pov: Option<String>,
    tone: Option<Vec<String>>,
    audience: Option<String>,
) -> Result<ProjectSnapshot, AppError> {
    let project_path = PathBuf::from(&path);
    ensure_dir(&project_path)?;

    let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);
    ensure_dir(&manuscripts_path)?;

    let normalized_cover_image =
        cover_image.and_then(|ci| if ci.trim().is_empty() { None } else { Some(ci) });

    let mut project = ProjectMetadata::new(name, author, project_type, normalized_cover_image);
    project.apply_creation_inputs(
        description,
        target_total_words,
        planned_volumes,
        target_words_per_volume,
        target_words_per_chapter,
        narrative_pov,
        tone,
        audience,
    );
    let project_file = project_path.join(PROJECT_FILE);
    write_json(&project_file, &project)?;
    crate::services::ensure_project_scaffold(&project_path, &project)?;

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
    let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);

    let mut project: ProjectMetadata = read_json(&project_file)?;
    project.mark_opened_now();

    // Migrate old manuscripts layout (title-based paths) to ID-based layout.
    crate::services::migrate_manuscripts_to_id_layout(&project_path)?;
    ensure_dir(&manuscripts_path)?;
    crate::services::ensure_project_scaffold(&project_path, &project)?;
    write_json(&project_file, &project)?;

    let tree = build_file_tree(&manuscripts_path, "")?;

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
                    Ok(mut project) => {
                        project.ensure_defaults();
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
    project.ensure_defaults();

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn create_project_creates_knowledge_scaffold() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("novel");

        let snapshot = create_project(
            project_path.to_string_lossy().to_string(),
            "Novel".to_string(),
            "Tester".to_string(),
            None,
            None,
            Some("A quiet mystery".to_string()),
            Some(210_000),
            Some(3),
            Some(70_000),
            Some(3_500),
            Some("first_person".to_string()),
            Some(vec!["tense".to_string()]),
            Some("young_adult".to_string()),
        )
        .await
        .expect("create");

        let scaffold_root =
            project_path.join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);
        assert_eq!(snapshot.path, project_path.to_string_lossy().to_string());
        assert!(project_path.join(PROJECT_FILE).exists());
        assert!(project_path.join(MANUSCRIPTS_DIR).is_dir());
        assert!(scaffold_root.join("guidelines.md").exists());
        assert!(scaffold_root.join("characters").is_dir());
        assert!(scaffold_root.join("terms").is_dir());
        assert!(scaffold_root.join("settings").is_dir());
        assert!(scaffold_root.join("world").is_dir());
        assert!(scaffold_root.join("plot").is_dir());
        assert!(scaffold_root.join("system/project_profile.md").exists());
        assert!(scaffold_root.join("planning/story_blueprint.md").exists());
        assert!(scaffold_root.join("index/object_index.json").exists());
        assert_eq!(snapshot.project.target_total_words, 210_000);
        assert_eq!(snapshot.project.planned_volumes, 3);
        assert_eq!(snapshot.project.target_words_per_volume, 70_000);
        assert_eq!(snapshot.project.target_words_per_chapter, 3_500);
        assert_eq!(snapshot.project.narrative_pov, "first_person");
        assert_eq!(snapshot.project.tone, vec!["tense".to_string()]);
        assert_eq!(snapshot.project.audience, "young_adult");
    }

    #[tokio::test]
    async fn open_project_scaffolds_without_overwriting_guidelines() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("project");
        ensure_dir(&project_path).expect("project dir");
        ensure_dir(&project_path.join(MANUSCRIPTS_DIR)).expect("manuscripts");

        let project = ProjectMetadata::new(
            "Current".to_string(),
            "Tester".to_string(),
            Some(vec!["fantasy".to_string()]),
            None,
        );
        write_json(&project_path.join(PROJECT_FILE), &project).expect("project file");

        let scaffold_root =
            project_path.join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);
        ensure_dir(&scaffold_root).expect("scaffold root");
        crate::services::write_file(
            &scaffold_root.join("guidelines.md"),
            "# Existing Guidelines\n",
        )
        .expect("guidelines");

        open_project(project_path.to_string_lossy().to_string())
            .await
            .expect("open");

        assert_eq!(
            std::fs::read_to_string(scaffold_root.join("guidelines.md")).expect("guidelines"),
            "# Existing Guidelines\n"
        );
        assert!(scaffold_root.join("characters").is_dir());
        assert!(scaffold_root.join("terms").is_dir());
        assert!(scaffold_root.join("settings").is_dir());
        assert!(scaffold_root.join("system/project_profile.md").exists());
        assert!(scaffold_root
            .join("task/current_bootstrap_task.md")
            .exists());

        let saved: ProjectMetadata = read_json(&project_path.join(PROJECT_FILE)).expect("project");
        assert_eq!(
            saved.bootstrap_state,
            crate::models::ProjectBootstrapState::ScaffoldReady
        );
        assert!(saved.last_opened_at.is_some());
    }
}
