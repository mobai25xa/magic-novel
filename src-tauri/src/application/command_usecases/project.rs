use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::command;

use crate::application::command_usecases::inspiration::{
    ConsensusFieldId, CreateProjectHandoffDraft, InspirationConsensusState,
};
use crate::application::command_usecases::planning_bundle::{
    ensure_planning_manifest_on_open, persist_planning_bundle,
};
use crate::application::command_usecases::planning_generation::{
    generate_planning_bundle, generate_planning_bundle_with_config,
};
use crate::models::{
    AppError, Chapter, ErrorCode, FileNode, PlanningManifest, ProjectMetadata, ProjectSnapshot,
    VolumeMetadata, PLANNING_BUNDLE_VERSION,
};
use crate::services::ai_settings::ResolvedPlanningGenerationConfig;
use crate::services::{ensure_dir, list_dirs, list_files, read_json, write_json};

const PROJECT_FILE: &str = "project.json";
const VOLUME_FILE: &str = "volume.json";
const MANUSCRIPTS_DIR: &str = "manuscripts";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectHomeAction {
    pub action: String,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectFromIdeationOutput {
    pub project_snapshot: ProjectSnapshot,
    pub planning_manifest: PlanningManifest,
    pub project_home_actions: Vec<ProjectHomeAction>,
}

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
        tree: Vec::new(),
    })
}

#[command]
pub async fn create_project_from_ideation(
    path: String,
    name: String,
    author: String,
    consensus_snapshot: InspirationConsensusState,
    create_handoff: CreateProjectHandoffDraft,
    origin_inspiration_session_id: Option<String>,
) -> Result<CreateProjectFromIdeationOutput, AppError> {
    create_project_from_ideation_impl(
        path,
        name,
        author,
        consensus_snapshot,
        create_handoff,
        origin_inspiration_session_id,
        None,
    )
    .await
}

async fn create_project_from_ideation_impl(
    path: String,
    name: String,
    author: String,
    consensus_snapshot: InspirationConsensusState,
    create_handoff: CreateProjectHandoffDraft,
    origin_inspiration_session_id: Option<String>,
    planning_config_override: Option<ResolvedPlanningGenerationConfig>,
) -> Result<CreateProjectFromIdeationOutput, AppError> {
    let final_path = PathBuf::from(&path);
    guard_project_destination(&final_path)?;

    let parent = final_path.parent().ok_or_else(|| {
        AppError::invalid_argument(format!(
            "invalid project path: {}",
            final_path.to_string_lossy()
        ))
    })?;
    ensure_dir(parent).map_err(persistence_error)?;

    let staging_path = staging_project_path(&final_path)?;
    if staging_path.exists() {
        cleanup_dir_if_exists(&staging_path);
    }
    ensure_dir(&staging_path).map_err(persistence_error)?;

    let creation_result = create_project_from_ideation_inner(
        &staging_path,
        &final_path,
        name,
        author,
        consensus_snapshot,
        create_handoff,
        origin_inspiration_session_id,
        planning_config_override,
    )
    .await;

    match creation_result {
        Ok(output) => Ok(output),
        Err(error) => {
            cleanup_dir_if_exists(&staging_path);
            Err(error)
        }
    }
}

#[command]
pub async fn open_project(path: String) -> Result<ProjectSnapshot, AppError> {
    let project_path = PathBuf::from(&path);
    let project_file = project_path.join(PROJECT_FILE);
    let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);

    let mut project: ProjectMetadata = read_json(&project_file)?;
    project.mark_opened_now();

    crate::services::migrate_manuscripts_to_id_layout(&project_path)?;
    ensure_dir(&manuscripts_path)?;
    crate::services::ensure_project_scaffold(&project_path, &project)?;
    let _ = ensure_planning_manifest_on_open(&project_path)?;
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
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    if let Ok(dirs) = list_dirs(&root_path) {
        for dir_name in dirs {
            let project_path = root_path.join(&dir_name);
            let project_file = project_path.join(PROJECT_FILE);

            if !project_file.exists() {
                continue;
            }

            match read_json::<ProjectMetadata>(&project_file) {
                Ok(mut project) => {
                    project.ensure_defaults();
                    let tree = build_file_tree(&project_path.join(MANUSCRIPTS_DIR), "")
                        .unwrap_or_default();
                    projects.push(ProjectSnapshot {
                        project,
                        path: project_path.to_string_lossy().to_string(),
                        tree,
                    });
                }
                Err(_) => continue,
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

    if let Some(name) = name {
        project.name = name;
    }
    if let Some(author) = author {
        project.author = author;
    }
    if description.is_some() {
        project.description = description.and_then(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    }
    if let Some(project_type) = project_type {
        let mut out = Vec::new();
        for item in project_type {
            let trimmed = item.trim().to_string();
            if trimmed.is_empty() || out.contains(&trimmed) {
                continue;
            }
            out.push(trimmed);
        }
        project.project_type = out;
    }
    if let Some(cover_image) = cover_image {
        project.cover_image = if cover_image.trim().is_empty() {
            None
        } else {
            Some(cover_image)
        };
    }

    project.updated_at = chrono::Utc::now().timestamp_millis();
    write_json(&project_file, &project)?;

    Ok(project)
}

async fn create_project_from_ideation_inner(
    staging_path: &Path,
    final_path: &Path,
    name: String,
    author: String,
    consensus_snapshot: InspirationConsensusState,
    create_handoff: CreateProjectHandoffDraft,
    origin_inspiration_session_id: Option<String>,
    planning_config_override: Option<ResolvedPlanningGenerationConfig>,
) -> Result<CreateProjectFromIdeationOutput, AppError> {
    let project_name = first_non_empty([name.as_str(), create_handoff.name.as_str()])
        .ok_or_else(|| AppError::invalid_argument("project name cannot be empty"))?;
    let author = first_non_empty([author.as_str()])
        .ok_or_else(|| AppError::invalid_argument("author cannot be empty"))?;

    ensure_dir(&staging_path.join(MANUSCRIPTS_DIR)).map_err(persistence_error)?;

    let mut project = ProjectMetadata::new(
        project_name,
        author,
        Some(create_handoff.project_type.clone()),
        None,
    );
    project.apply_creation_inputs(
        Some(create_handoff.description.clone()),
        None,
        None,
        None,
        None,
        None,
        Some(create_handoff.tone.clone()),
        Some(create_handoff.audience.clone()),
    );
    project.story_core = consensus_snapshot.resolved_text(ConsensusFieldId::StoryCore);
    project.protagonist_anchor = consensus_snapshot.resolved_text(ConsensusFieldId::Protagonist);
    project.conflict_anchor = consensus_snapshot.resolved_text(ConsensusFieldId::CoreConflict);
    project.origin_inspiration_session_id = origin_inspiration_session_id.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    project.planning_bundle_version = Some(PLANNING_BUNDLE_VERSION);
    crate::services::ensure_project_scaffold(staging_path, &project).map_err(persistence_error)?;
    write_json(&staging_path.join(PROJECT_FILE), &project).map_err(persistence_error)?;

    let bundle = match planning_config_override.as_ref() {
        Some(config) => {
            generate_planning_bundle_with_config(
                &project,
                &consensus_snapshot,
                &create_handoff,
                config,
            )
            .await?
        }
        None => generate_planning_bundle(&project, &consensus_snapshot, &create_handoff).await?,
    };
    persist_planning_bundle(staging_path, &bundle).map_err(persistence_error)?;

    std::fs::rename(staging_path, final_path).map_err(|error| {
        persistence_error(AppError {
            code: ErrorCode::IoError,
            message: format!(
                "failed to finalize project creation at {}: {error}",
                final_path.to_string_lossy()
            ),
            details: None,
            recoverable: Some(false),
        })
    })?;

    let tree = build_file_tree(&final_path.join(MANUSCRIPTS_DIR), "")?;
    let project_snapshot = ProjectSnapshot {
        project,
        path: final_path.to_string_lossy().to_string(),
        tree,
    };

    Ok(CreateProjectFromIdeationOutput {
        planning_manifest: bundle.manifest.clone(),
        project_home_actions: build_project_home_actions(&bundle.manifest),
        project_snapshot,
    })
}

fn build_project_home_actions(manifest: &PlanningManifest) -> Vec<ProjectHomeAction> {
    vec![
        ProjectHomeAction {
            action: "continue_planning".to_string(),
            enabled: true,
            target_path: Some(manifest.recommended_next_doc.clone()),
        },
        ProjectHomeAction {
            action: "view_contracts".to_string(),
            enabled: true,
            target_path: Some(
                manifest
                    .docs
                    .first()
                    .map(|entry| entry.path.clone())
                    .unwrap_or_else(|| manifest.recommended_next_doc.clone()),
            ),
        },
        ProjectHomeAction {
            action: "start_writing".to_string(),
            enabled: manifest.writing_readiness.can_start,
            target_path: Some(manifest.recommended_next_doc.clone()),
        },
    ]
}

fn build_file_tree(base_path: &Path, relative_path: &str) -> Result<Vec<FileNode>, AppError> {
    let mut nodes = Vec::new();
    let dirs = list_dirs(base_path)?;

    for dir_name in dirs {
        let dir_path = base_path.join(&dir_name);
        let volume_file = dir_path.join(VOLUME_FILE);

        let rel_path = if relative_path.is_empty() {
            dir_name.clone()
        } else {
            format!("{relative_path}/{dir_name}")
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
    volume_path: &Path,
    relative_path: &str,
    volume: &VolumeMetadata,
) -> Result<Vec<FileNode>, AppError> {
    let mut nodes = Vec::new();
    let files = list_files(volume_path, ".json")?;
    let mut chapter_map: std::collections::HashMap<String, FileNode> =
        std::collections::HashMap::new();
    let mut unordered = Vec::new();

    for file_name in files {
        if file_name == VOLUME_FILE {
            continue;
        }

        let file_path = volume_path.join(&file_name);
        let chapter: Chapter = read_json(&file_path)?;
        let chapter_id = chapter.id.clone();
        let rel_path = format!("{relative_path}/{file_name}");

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
            status: chapter
                .status
                .map(|status| format!("{status:?}").to_lowercase()),
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

fn guard_project_destination(project_path: &Path) -> Result<(), AppError> {
    if project_path.exists() {
        return Err(AppError {
            code: ErrorCode::Conflict,
            message: format!(
                "project path already exists: {}",
                project_path.to_string_lossy()
            ),
            details: Some(json!({ "code": "PersistenceFailed" })),
            recoverable: Some(true),
        });
    }

    Ok(())
}

fn staging_project_path(final_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = final_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::invalid_argument(format!(
                "invalid project path: {}",
                final_path.to_string_lossy()
            ))
        })?;
    let parent = final_path.parent().ok_or_else(|| {
        AppError::invalid_argument(format!(
            "invalid project path: {}",
            final_path.to_string_lossy()
        ))
    })?;

    Ok(parent.join(format!("{file_name}.creating.{}", uuid::Uuid::new_v4())))
}

fn cleanup_dir_if_exists(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

fn persistence_error(error: AppError) -> AppError {
    if has_domain_code(&error, "MissingMinimumConsensus")
        || has_domain_code(&error, "CoreBundleGenerationFailed")
        || has_domain_code(&error, "PlanningProviderConfigurationInvalid")
    {
        return error;
    }

    AppError {
        code: error.code,
        message: error.message,
        details: Some(json!({ "code": "PersistenceFailed" })),
        recoverable: error.recoverable,
    }
}

fn has_domain_code(error: &AppError, expected: &str) -> bool {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("code"))
        .and_then(|code| code.as_str())
        .map(|code| code == expected)
        .unwrap_or(false)
}

fn first_non_empty<'a, I>(values: I) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    values.into_iter().find_map(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::application::command_usecases::inspiration::{ConsensusField, ConsensusValue};
    use crate::services::ai_settings::ResolvedPlanningGenerationConfig;

    fn field(field_id: ConsensusFieldId, value: ConsensusValue) -> ConsensusField {
        ConsensusField {
            field_id,
            draft_value: Some(value),
            confirmed_value: None,
            locked: false,
            updated_at: 1,
            last_source_turn_id: None,
        }
    }

    fn valid_consensus() -> InspirationConsensusState {
        let mut state = InspirationConsensusState::default();
        state.story_core = field(
            ConsensusFieldId::StoryCore,
            ConsensusValue::Text("秘密交易撬动旧秩序".to_string()),
        );
        state.premise = field(
            ConsensusFieldId::Premise,
            ConsensusValue::Text("一个习惯自保的人被迫卷入会吞噬身份的交易网络".to_string()),
        );
        state.genre_tone = field(
            ConsensusFieldId::GenreTone,
            ConsensusValue::List(vec!["悬疑".to_string(), "压迫感".to_string()]),
        );
        state.protagonist = field(
            ConsensusFieldId::Protagonist,
            ConsensusValue::Text("沈砚".to_string()),
        );
        state.core_conflict = field(
            ConsensusFieldId::CoreConflict,
            ConsensusValue::Text("想查明真相就必须继续喂养那套危险规则".to_string()),
        );
        state.audience = field(
            ConsensusFieldId::Audience,
            ConsensusValue::Text("偏好强情节女性向悬疑的读者".to_string()),
        );
        state
    }

    fn valid_handoff() -> CreateProjectHandoffDraft {
        CreateProjectHandoffDraft {
            name: "暗潮协议".to_string(),
            description: "一个习惯自保的人被迫卷入会吞噬身份的交易网络".to_string(),
            project_type: vec!["悬疑".to_string()],
            tone: vec!["压迫".to_string()],
            audience: "偏好强情节女性向悬疑的读者".to_string(),
            protagonist_seed: Some("沈砚，擅长隐藏真实意图".to_string()),
            counterpart_seed: None,
            world_seed: None,
            ending_direction: Some("主角必须亲手切断最诱人的捷径".to_string()),
        }
    }

    fn deterministic_planning_config() -> ResolvedPlanningGenerationConfig {
        ResolvedPlanningGenerationConfig {
            mode: "follow_primary".to_string(),
            provider_type: "openai-compatible".to_string(),
            model: "gpt-5".to_string(),
            base_url: String::new(),
            api_key: String::new(),
            source_tag: "deterministic_fallback".to_string(),
            can_use_llm: false,
        }
    }

    #[tokio::test]
    async fn create_project_creates_minimal_knowledge_scaffold() {
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
        assert!(scaffold_root.join("planning").is_dir());
        assert!(scaffold_root.join("foreshadow").is_dir());
        assert!(scaffold_root.join("index/object_index.json").exists());
        assert!(!scaffold_root.join("system/project_profile.md").exists());
        assert!(!scaffold_root.join("planning/story_blueprint.md").exists());
        assert_eq!(snapshot.project.target_total_words, 210_000);
        assert_eq!(snapshot.project.planned_volumes, Some(3));
        assert_eq!(snapshot.project.target_words_per_volume, Some(70_000));
        assert_eq!(snapshot.project.target_words_per_chapter, Some(3_500));
    }

    #[tokio::test]
    async fn create_project_from_ideation_creates_core_bundle_and_manifest() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("novel");

        let output = create_project_from_ideation_impl(
            project_path.to_string_lossy().to_string(),
            "暗潮协议".to_string(),
            "Tester".to_string(),
            valid_consensus(),
            valid_handoff(),
            Some("session-1".to_string()),
            Some(deterministic_planning_config()),
        )
        .await
        .expect("create");

        let scaffold_root =
            project_path.join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY);
        assert!(project_path.join(PROJECT_FILE).exists());
        assert!(scaffold_root.join("planning/story_brief.md").exists());
        assert!(scaffold_root.join("planning/story_blueprint.md").exists());
        assert!(scaffold_root
            .join("planning/narrative_contract.md")
            .exists());
        assert!(scaffold_root.join("planning/character_cards.md").exists());
        assert!(scaffold_root
            .join("planning/foreshadow_registry.md")
            .exists());
        assert!(scaffold_root.join("planning/chapter_planning.md").exists());
        assert!(scaffold_root.join("planning/index.json").exists());
        assert!(!scaffold_root
            .join("task/current_bootstrap_task.md")
            .exists());
        assert!(output.project_snapshot.tree.is_empty());
        assert_eq!(output.project_snapshot.project.planned_volumes, None);
        assert_eq!(
            output
                .project_snapshot
                .project
                .origin_inspiration_session_id
                .as_deref(),
            Some("session-1")
        );
        assert!(!output.planning_manifest.writing_readiness.can_start);
        assert_eq!(
            output.planning_manifest.generation_source.as_deref(),
            Some("deterministic_fallback")
        );
        assert_eq!(output.planning_manifest.generation_provider, None);
        assert_eq!(output.planning_manifest.generation_model, None);
        assert_eq!(output.project_home_actions[0].action, "continue_planning");
        assert_eq!(
            output.project_home_actions[2].enabled,
            output.planning_manifest.writing_readiness.can_start
        );
    }

    #[tokio::test]
    async fn create_project_from_ideation_rejects_missing_consensus_without_leaving_project() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("novel");

        let err = create_project_from_ideation_impl(
            project_path.to_string_lossy().to_string(),
            "暗潮协议".to_string(),
            "Tester".to_string(),
            InspirationConsensusState::default(),
            valid_handoff(),
            Some("session-1".to_string()),
            Some(deterministic_planning_config()),
        )
        .await
        .expect_err("missing consensus");

        assert!(has_domain_code(&err, "MissingMinimumConsensus"));
        assert!(!project_path.exists());
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
        assert!(!scaffold_root.join("system/project_profile.md").exists());

        let saved: ProjectMetadata = read_json(&project_path.join(PROJECT_FILE)).expect("project");
        assert_eq!(
            saved.bootstrap_state,
            crate::models::ProjectBootstrapState::ScaffoldReady
        );
        assert!(saved.last_opened_at.is_some());
    }

    #[tokio::test]
    async fn open_project_upgrades_legacy_project_schema() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("project");
        ensure_dir(&project_path).expect("project dir");
        ensure_dir(&project_path.join(MANUSCRIPTS_DIR)).expect("manuscripts");

        crate::services::write_file(
            &project_path.join(PROJECT_FILE),
            &serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 2,
                "project_id": "legacy-project",
                "name": "Legacy",
                "author": "Tester",
                "project_type": "mystery",
                "created_at": 100,
                "updated_at": 200,
                "last_opened_at": 300
            }))
            .expect("legacy project json"),
        )
        .expect("legacy project file");

        let snapshot = open_project(project_path.to_string_lossy().to_string())
            .await
            .expect("open");

        assert_eq!(
            snapshot.project.schema_version,
            crate::models::PROJECT_SCHEMA_VERSION
        );
        assert_eq!(snapshot.project.project_type, vec!["mystery".to_string()]);
        assert_eq!(snapshot.project.planned_volumes, None);
        assert_eq!(
            snapshot.project.bootstrap_state,
            crate::models::ProjectBootstrapState::ScaffoldReady
        );
    }

    #[tokio::test]
    async fn open_project_builds_manifest_for_legacy_bootstrap_outputs() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("project");
        ensure_dir(&project_path).expect("project dir");
        ensure_dir(&project_path.join(MANUSCRIPTS_DIR)).expect("manuscripts");

        let project = ProjectMetadata::new(
            "Legacy".to_string(),
            "Tester".to_string(),
            Some(vec!["mystery".to_string()]),
            None,
        );
        write_json(&project_path.join(PROJECT_FILE), &project).expect("project file");
        crate::services::write_file(
            &project_path.join(".magic_novel/system/creation_brief.md"),
            "# Creation Brief\n\n一个总想自保的人被迫进入会吞噬身份的交易网络。\n",
        )
        .expect("creation brief");
        crate::services::write_file(
            &project_path.join(".magic_novel/planning/story_blueprint.md"),
            "# Story Blueprint\n\n主角必须在每次靠近真相时承担更高的身份代价。\n",
        )
        .expect("story blueprint");
        crate::services::write_file(
            &project_path.join(".magic_novel/characters/protagonist.md"),
            "# Protagonist\n\n沈砚，擅长隐藏意图，但已经没有办法继续旁观。\n",
        )
        .expect("protagonist");
        crate::services::write_file(
            &project_path.join(".magic_novel/planning/chapter_backlog.md"),
            "# Chapter Backlog\n\n- 第1章：异常信号进入视野。\n",
        )
        .expect("chapter backlog");

        open_project(project_path.to_string_lossy().to_string())
            .await
            .expect("open");

        let manifest: PlanningManifest =
            read_json(&project_path.join(crate::models::PLANNING_MANIFEST_REL_PATH))
                .expect("manifest");

        assert_eq!(
            manifest
                .doc(crate::models::PlanningDocId::StoryBrief)
                .expect("story brief")
                .materialization_state,
            crate::models::MaterializationState::Ready
        );
        assert_eq!(
            manifest
                .doc(crate::models::PlanningDocId::ChapterPlanning)
                .expect("chapter planning")
                .materialization_state,
            crate::models::MaterializationState::Ready
        );
        assert_eq!(
            manifest.recommended_next_doc,
            crate::models::PlanningDocId::NarrativeContract.relative_path()
        );
    }

    #[tokio::test]
    async fn open_project_upgrades_legacy_volume_schema() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("project");
        let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);
        let volume_dir = manuscripts_path.join("volume-1");
        let volume_file = volume_dir.join(VOLUME_FILE);
        ensure_dir(&volume_dir).expect("volume dir");

        let project = ProjectMetadata::new(
            "Current".to_string(),
            "Tester".to_string(),
            Some(vec!["fantasy".to_string()]),
            None,
        );
        write_json(&project_path.join(PROJECT_FILE), &project).expect("project file");

        crate::services::write_file(
            &volume_file,
            &serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "volume_id": "volume-1",
                "title": "卷一",
                "created_at": 100,
                "updated_at": 200
            }))
            .expect("legacy volume json"),
        )
        .expect("legacy volume file");

        let snapshot = open_project(project_path.to_string_lossy().to_string())
            .await
            .expect("open");

        assert_eq!(snapshot.tree.len(), 1);
    }

    #[tokio::test]
    async fn open_project_upgrades_legacy_chapter_schema() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().join("project");
        let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);
        let volume = VolumeMetadata::new("卷一".to_string());
        let volume_dir = manuscripts_path.join(&volume.volume_id);
        let chapter_file = volume_dir.join("chapter-1.json");
        ensure_dir(&volume_dir).expect("volume dir");

        let project = ProjectMetadata::new(
            "Current".to_string(),
            "Tester".to_string(),
            Some(vec!["fantasy".to_string()]),
            None,
        );
        write_json(&project_path.join(PROJECT_FILE), &project).expect("project file");
        write_json(&volume_dir.join(VOLUME_FILE), &volume).expect("volume file");

        crate::services::write_file(
            &chapter_file,
            &serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "id": "chapter-1",
                "title": "第一章",
                "content": { "type": "doc", "content": [] },
                "counts": {
                    "text_length_no_whitespace": 0,
                    "word_count": serde_json::Value::Null,
                    "algorithm_version": 1,
                    "last_calculated_at": 100
                },
                "status": "draft",
                "created_at": 100,
                "updated_at": 200
            }))
            .expect("legacy chapter json"),
        )
        .expect("legacy chapter file");

        let snapshot = open_project(project_path.to_string_lossy().to_string())
            .await
            .expect("open");

        assert_eq!(snapshot.tree.len(), 1);
    }
}
