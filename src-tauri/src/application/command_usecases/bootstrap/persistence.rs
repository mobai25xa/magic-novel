use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::llm::bootstrap::{BootstrapArtifactKind, BootstrapPromptInput};
use crate::models::{AppError, ErrorCode, ProjectBootstrapState, ProjectMetadata};
use crate::services::{ensure_dir, read_json, write_file, write_json};
use crate::utils::atomic_write::atomic_write_json;

use super::types::{ProjectBootstrapPhase, ProjectBootstrapStatus};

const PROJECT_FILE: &str = "project.json";
const BOOTSTRAP_STATUS_FILE: &str = "bootstrap_status.json";
const BOOTSTRAP_REQUEST_FILE: &str = "bootstrap_request.json";
const PROJECT_PROFILE_PATH: &str = "system/project_profile.md";
const CREATION_BRIEF_PATH: &str = "system/creation_brief.md";
const CURRENT_BOOTSTRAP_TASK_PATH: &str = "task/current_bootstrap_task.md";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedBootstrapRequest {
    pub schema_version: i32,
    pub project_id: String,
    pub prompt_input: BootstrapPromptInput,
    pub requested_kinds: Vec<BootstrapArtifactKind>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl PersistedBootstrapRequest {
    pub fn new(
        project_id: String,
        prompt_input: BootstrapPromptInput,
        requested_kinds: Vec<BootstrapArtifactKind>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: 1,
            project_id,
            prompt_input,
            requested_kinds,
            created_at: now,
            updated_at: now,
        }
    }
}

pub fn load_request(project_path: &Path) -> Result<PersistedBootstrapRequest, AppError> {
    read_json(&request_file_path(project_path))
}

pub fn save_request(
    project_path: &Path,
    request: &PersistedBootstrapRequest,
    project: &ProjectMetadata,
) -> Result<(), AppError> {
    let request_path = request_file_path(project_path);
    ensure_parent(&request_path)?;
    atomic_write_json(&request_path, request)?;

    write_file(
        &knowledge_file_path(project_path, PROJECT_PROFILE_PATH),
        &render_project_profile(project),
    )?;
    write_file(
        &knowledge_file_path(project_path, CREATION_BRIEF_PATH),
        &render_creation_brief(request),
    )?;

    Ok(())
}

pub fn load_status(project_path: &Path) -> Result<ProjectBootstrapStatus, AppError> {
    read_json(&status_file_path(project_path))
}

pub fn save_status(project_path: &Path, status: &ProjectBootstrapStatus) -> Result<(), AppError> {
    let status_path = status_file_path(project_path);
    ensure_parent(&status_path)?;
    atomic_write_json(&status_path, status)?;
    write_file(
        &knowledge_file_path(project_path, CURRENT_BOOTSTRAP_TASK_PATH),
        &render_current_bootstrap_task(status),
    )?;
    sync_project_bootstrap_state(project_path, status)
}

pub fn status_file_path(project_path: &Path) -> PathBuf {
    knowledge_file_path(project_path, &format!("system/{BOOTSTRAP_STATUS_FILE}"))
}

pub fn request_file_path(project_path: &Path) -> PathBuf {
    knowledge_file_path(project_path, &format!("system/{BOOTSTRAP_REQUEST_FILE}"))
}

fn sync_project_bootstrap_state(
    project_path: &Path,
    status: &ProjectBootstrapStatus,
) -> Result<(), AppError> {
    let project_file = project_path.join(PROJECT_FILE);
    if !project_file.exists() {
        return Err(AppError::not_found(format!(
            "project file not found: {}",
            project_file.to_string_lossy()
        )));
    }

    let mut project: ProjectMetadata = read_json(&project_file)?;
    project.bootstrap_state = map_phase_to_project_state(&status.phase);
    project.bootstrap_updated_at = status.updated_at;
    project.updated_at = status.updated_at;
    write_json(&project_file, &project)
}

fn map_phase_to_project_state(phase: &ProjectBootstrapPhase) -> ProjectBootstrapState {
    match phase {
        ProjectBootstrapPhase::Pending
        | ProjectBootstrapPhase::AssemblingPrompt
        | ProjectBootstrapPhase::LlmGenerating
        | ProjectBootstrapPhase::WritingArtifacts => ProjectBootstrapState::BootstrapRunning,
        ProjectBootstrapPhase::PartiallyGenerated => ProjectBootstrapState::PartiallyGenerated,
        ProjectBootstrapPhase::ReadyForReview => ProjectBootstrapState::ReadyForReview,
        ProjectBootstrapPhase::ReadyToWrite => ProjectBootstrapState::ReadyToWrite,
        ProjectBootstrapPhase::Failed => ProjectBootstrapState::Failed,
    }
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
        project.bootstrap_state
    )
}

fn render_creation_brief(request: &PersistedBootstrapRequest) -> String {
    let input = &request.prompt_input;
    let tone = display_list(&input.tone);
    let genres = display_list(&input.genres);
    format!(
        "# Creation Brief\n\n## Project\n- Name: {}\n- Author: {}\n- Genres: {}\n- Target Total Words: {}\n- Planned Volumes: {}\n- Target Words Per Volume: {}\n- Target Words Per Chapter: {}\n- POV: {}\n- Tone: {}\n- Audience: {}\n\n## Brief\n{}\n\n## Seeds\n- Protagonist: {}\n- Counterpart: {}\n- World: {}\n- Ending Direction: {}\n",
        input.project_name,
        input.author,
        genres,
        input.target_total_words,
        input.planned_volumes,
        input.target_words_per_volume,
        input.target_words_per_chapter,
        input.narrative_pov,
        tone,
        input.audience,
        input.creation_brief,
        input.protagonist_seed.as_deref().unwrap_or("待生成"),
        input.counterpart_seed.as_deref().unwrap_or("待生成"),
        input.world_seed.as_deref().unwrap_or("待生成"),
        input.ending_direction.as_deref().unwrap_or("待生成"),
    )
}

fn render_current_bootstrap_task(status: &ProjectBootstrapStatus) -> String {
    let completed = if status.completed_steps.is_empty() {
        "- none".to_string()
    } else {
        status
            .completed_steps
            .iter()
            .map(|step| format!("- {step}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let failed = if status.failed_steps.is_empty() {
        "- none".to_string()
    } else {
        status
            .failed_steps
            .iter()
            .map(|step| format!("- {step}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut output = format!(
        "# Current Bootstrap Task\n\n- job_id: {}\n- phase: {:?}\n- progress: {}\n- next: {}\n\n## Completed Steps\n{}\n\n## Failed Steps\n{}\n",
        status.creation_job_id,
        status.phase,
        status.progress,
        status.recommended_next_action,
        completed,
        failed
    );

    if let Some(message) = &status.error_message {
        output.push_str(&format!("\n## Error\n{message}\n"));
    }

    output
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() {
        "待补充".to_string()
    } else {
        values.join(" / ")
    }
}

fn knowledge_file_path(project_path: &Path, relative_path: &str) -> PathBuf {
    project_path
        .join(crate::services::knowledge_paths::KNOWLEDGE_ROOT_PRIMARY)
        .join(relative_path)
}

fn ensure_parent(path: &Path) -> Result<(), AppError> {
    let Some(parent) = path.parent() else {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: format!("missing parent path for {}", path.to_string_lossy()),
            details: None,
            recoverable: Some(false),
        });
    };
    ensure_dir(parent)
}
