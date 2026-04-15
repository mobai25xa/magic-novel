use std::path::Path;

use crate::models::{AppError, ProjectMetadata};
use crate::services::write_json;

const OBJECT_INDEX_PATH: &str = "index/object_index.json";

pub fn ensure_project_scaffold(
    project_path: &Path,
    _project: &ProjectMetadata,
) -> Result<(), AppError> {
    crate::services::knowledge_paths::ensure_project_knowledge_scaffold(project_path)?;

    let root = project_path.join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);
    let object_index_path = root.join(OBJECT_INDEX_PATH);
    if !object_index_path.exists() {
        write_json(
            &object_index_path,
            &serde_json::json!({
                "schema_version": 1,
                "objects": []
            }),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::services::read_file;

    fn build_project() -> ProjectMetadata {
        let mut project = ProjectMetadata::new(
            "Project X".to_string(),
            "Tester".to_string(),
            Some(vec!["fantasy".to_string()]),
            None,
        );
        project.apply_creation_inputs(
            Some("A mysterious opening.".to_string()),
            Some(180_000),
            Some(3),
            Some(60_000),
            Some(4_000),
            Some("third_person".to_string()),
            Some(vec!["moody".to_string()]),
            Some("female".to_string()),
        );
        project
    }

    #[test]
    fn ensure_project_scaffold_creates_required_dirs_and_index_only() {
        let dir = tempdir().expect("temp");
        let project = build_project();

        ensure_project_scaffold(dir.path(), &project).expect("scaffold");

        let root = dir
            .path()
            .join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);
        for path in [
            "characters",
            "terms",
            "settings",
            "planning",
            "foreshadow",
            "index",
            "system",
            "task",
        ] {
            assert!(root.join(path).is_dir(), "missing dir: {path}");
        }

        assert!(root.join("guidelines.md").exists());
        assert!(root.join(OBJECT_INDEX_PATH).exists());
        assert!(!root.join("system/project_profile.md").exists());
        assert!(!root.join("planning/story_blueprint.md").exists());
    }

    #[test]
    fn ensure_project_scaffold_does_not_overwrite_existing_index() {
        let dir = tempdir().expect("temp");
        let project = build_project();
        let root = dir
            .path()
            .join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);

        crate::services::ensure_dir(&root.join("index")).expect("index");
        crate::services::write_file(&root.join(OBJECT_INDEX_PATH), "{\"schema_version\":99}\n")
            .expect("index");

        ensure_project_scaffold(dir.path(), &project).expect("scaffold");

        assert_eq!(
            read_file(&root.join(OBJECT_INDEX_PATH)).expect("index"),
            "{\"schema_version\":99}\n"
        );
    }
}
