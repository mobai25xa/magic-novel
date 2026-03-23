use std::path::Path;

use crate::models::{AppError, ProjectMetadata};
use crate::services::{ensure_dir, write_file, write_json};

const ROOT_DIRS: &[&str] = &[
    "characters",
    "terms",
    "settings",
    "world",
    "plot",
    "system",
    "style",
    "rules",
    "planning",
    "task",
    "index",
];

const PROJECT_PROFILE_PATH: &str = "system/project_profile.md";
const CREATION_BRIEF_PATH: &str = "system/creation_brief.md";
const STYLE_GUIDE_PATH: &str = "style/style_guide.md";
const WRITING_RULES_PATH: &str = "rules/writing_rules.md";
const STORY_BLUEPRINT_PATH: &str = "planning/story_blueprint.md";
const VOLUME_PLAN_PATH: &str = "planning/volume_plan.md";
const CHAPTER_BACKLOG_PATH: &str = "planning/chapter_backlog.md";
const CURRENT_BOOTSTRAP_TASK_PATH: &str = "task/current_bootstrap_task.md";
const OBJECT_INDEX_PATH: &str = "index/object_index.json";

pub fn ensure_project_scaffold(
    project_path: &Path,
    project: &ProjectMetadata,
) -> Result<(), AppError> {
    crate::services::knowledge_paths::ensure_project_knowledge_scaffold(project_path)?;

    let root = project_path.join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);
    for dir in ROOT_DIRS {
        ensure_dir(&root.join(dir))?;
    }

    write_missing_text(
        &root.join(PROJECT_PROFILE_PATH),
        &render_project_profile(project),
    )?;
    write_missing_text(
        &root.join(CREATION_BRIEF_PATH),
        &render_creation_brief(project),
    )?;
    write_missing_text(&root.join(STYLE_GUIDE_PATH), STYLE_GUIDE_TEMPLATE)?;
    write_missing_text(&root.join(WRITING_RULES_PATH), WRITING_RULES_TEMPLATE)?;
    write_missing_text(&root.join(STORY_BLUEPRINT_PATH), STORY_BLUEPRINT_TEMPLATE)?;
    write_missing_text(&root.join(VOLUME_PLAN_PATH), VOLUME_PLAN_TEMPLATE)?;
    write_missing_text(&root.join(CHAPTER_BACKLOG_PATH), CHAPTER_BACKLOG_TEMPLATE)?;
    write_missing_text(
        &root.join(CURRENT_BOOTSTRAP_TASK_PATH),
        &render_current_bootstrap_task(project),
    )?;
    write_missing_json(
        &root.join(OBJECT_INDEX_PATH),
        &serde_json::json!({
            "schema_version": 1,
            "objects": []
        }),
    )?;

    Ok(())
}

fn write_missing_text(path: &Path, content: &str) -> Result<(), AppError> {
    if !path.exists() {
        write_file(path, content)?;
    }
    Ok(())
}

fn write_missing_json(path: &Path, value: &serde_json::Value) -> Result<(), AppError> {
    if !path.exists() {
        write_json(path, value)?;
    }
    Ok(())
}

fn render_project_profile(project: &ProjectMetadata) -> String {
    let description = project.description.as_deref().unwrap_or("待补充");
    let tone = if project.tone.is_empty() {
        "待补充".to_string()
    } else {
        project.tone.join(" / ")
    };

    format!(
        "# Project Profile\n\n- Name: {}\n- Author: {}\n- Description: {}\n- Genres: {}\n- Target Total Words: {}\n- Planned Volumes: {}\n- Target Words Per Volume: {}\n- Target Words Per Chapter: {}\n- Narrative POV: {}\n- Tone: {}\n- Audience: {}\n- Bootstrap State: {:?}\n",
        project.name,
        project.author,
        description,
        display_list(&project.project_type),
        project.target_total_words,
        project.planned_volumes,
        project.target_words_per_volume,
        project.target_words_per_chapter,
        project.narrative_pov,
        tone,
        project.audience,
        bootstrap_state_label(project.bootstrap_state)
    )
}

fn render_creation_brief(project: &ProjectMetadata) -> String {
    let description = project.description.as_deref().unwrap_or("待补充");

    format!(
        "# Creation Brief\n\n## Summary\n{}\n\n## Constraints\n- Total words: {}\n- Planned volumes: {}\n- Target words per chapter: {}\n- Narrative POV: {}\n- Audience: {}\n",
        description,
        project.target_total_words,
        project.planned_volumes,
        project.target_words_per_chapter,
        project.narrative_pov,
        project.audience
    )
}

fn render_current_bootstrap_task(project: &ProjectMetadata) -> String {
    format!(
        "# Current Bootstrap Task\n\n- status: scaffold_ready\n- next: start_project_bootstrap\n- project: {}\n- target_total_words: {}\n- planned_volumes: {}\n",
        project.name, project.target_total_words, project.planned_volumes
    )
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() {
        "待补充".to_string()
    } else {
        values.join(" / ")
    }
}

fn bootstrap_state_label(state: crate::models::ProjectBootstrapState) -> &'static str {
    match state {
        crate::models::ProjectBootstrapState::ScaffoldReady => "scaffold_ready",
        crate::models::ProjectBootstrapState::BootstrapRunning => "bootstrap_running",
        crate::models::ProjectBootstrapState::PartiallyGenerated => "partially_generated",
        crate::models::ProjectBootstrapState::ReadyForReview => "ready_for_review",
        crate::models::ProjectBootstrapState::ReadyToWrite => "ready_to_write",
        crate::models::ProjectBootstrapState::Failed => "failed",
    }
}

const STYLE_GUIDE_TEMPLATE: &str = "# Style Guide\n\n待补充。\n";
const WRITING_RULES_TEMPLATE: &str = "# Writing Rules\n\n待补充。\n";
const STORY_BLUEPRINT_TEMPLATE: &str = "# Story Blueprint\n\n待生成。\n";
const VOLUME_PLAN_TEMPLATE: &str = "# Volume Plan\n\n待生成。\n";
const CHAPTER_BACKLOG_TEMPLATE: &str = "# Chapter Backlog\n\n待生成。\n";

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
    fn ensure_project_scaffold_creates_required_dirs_and_files() {
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
            "world",
            "plot",
            "system",
            "style",
            "rules",
            "planning",
            "task",
            "index",
        ] {
            assert!(root.join(path).is_dir(), "missing dir: {path}");
        }

        for path in [
            "guidelines.md",
            PROJECT_PROFILE_PATH,
            CREATION_BRIEF_PATH,
            STYLE_GUIDE_PATH,
            WRITING_RULES_PATH,
            STORY_BLUEPRINT_PATH,
            VOLUME_PLAN_PATH,
            CHAPTER_BACKLOG_PATH,
            CURRENT_BOOTSTRAP_TASK_PATH,
            OBJECT_INDEX_PATH,
        ] {
            assert!(root.join(path).exists(), "missing file: {path}");
        }

        let profile = read_file(&root.join(PROJECT_PROFILE_PATH)).expect("profile");
        assert!(profile.contains("Project X"));
        assert!(profile.contains("180000"));
    }

    #[test]
    fn ensure_project_scaffold_does_not_overwrite_existing_templates() {
        let dir = tempdir().expect("temp");
        let project = build_project();
        let root = dir
            .path()
            .join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);

        ensure_dir(&root.join("system")).expect("system");
        write_file(&root.join(PROJECT_PROFILE_PATH), "# Existing Profile\n").expect("profile");
        ensure_dir(&root.join("index")).expect("index");
        write_file(&root.join(OBJECT_INDEX_PATH), "{\"schema_version\":99}\n").expect("index");

        ensure_project_scaffold(dir.path(), &project).expect("scaffold");

        assert_eq!(
            read_file(&root.join(PROJECT_PROFILE_PATH)).expect("profile"),
            "# Existing Profile\n"
        );
        assert_eq!(
            read_file(&root.join(OBJECT_INDEX_PATH)).expect("index"),
            "{\"schema_version\":99}\n"
        );
    }
}
