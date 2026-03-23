mod persistence;
mod runner;
mod types;

use std::path::PathBuf;

use tauri::command;

use crate::llm::bootstrap::{default_generator, BootstrapArtifactKind, BootstrapPromptInput};
use crate::models::{AppError, ErrorCode, ProjectMetadata};
use crate::services::{ensure_project_scaffold, read_json, write_json};

use self::persistence::{
    load_request, load_status, save_request, save_status, PersistedBootstrapRequest,
};
use self::runner::run_bootstrap_job;
pub use self::types::{
    ProjectBootstrapArtifact, ProjectBootstrapStatus, StartProjectBootstrapInput,
};

const PROJECT_FILE: &str = "project.json";

#[command]
pub async fn start_project_bootstrap(
    mut input: StartProjectBootstrapInput,
) -> Result<ProjectBootstrapStatus, AppError> {
    input.normalize();
    let project_path = PathBuf::from(&input.project_path);
    let mut project = load_project_metadata(&project_path)?;
    guard_against_active_job(&project_path)?;

    project.apply_creation_inputs(
        input.description.clone(),
        input.target_total_words,
        input.planned_volumes,
        input.target_words_per_volume,
        input.target_words_per_chapter,
        input.narrative_pov.clone(),
        input.tone.clone(),
        input.audience.clone(),
    );
    ensure_project_scaffold(&project_path, &project)?;
    write_json(&project_path.join(PROJECT_FILE), &project)?;

    let request = PersistedBootstrapRequest::new(
        project.project_id.clone(),
        BootstrapPromptInput::from_project(
            &project,
            input.creation_brief,
            input.protagonist_seed,
            input.counterpart_seed,
            input.world_seed,
            input.ending_direction,
        ),
        BootstrapArtifactKind::all().to_vec(),
    );
    save_request(&project_path, &request, &project)?;

    let status = ProjectBootstrapStatus::pending(
        project.project_id.clone(),
        uuid::Uuid::new_v4().to_string(),
    );
    save_status(&project_path, &status)?;
    spawn_bootstrap_job(
        project_path,
        request,
        BootstrapArtifactKind::all().to_vec(),
        status.clone(),
    );

    Ok(status)
}

#[command]
pub async fn get_project_bootstrap_status(
    project_path: String,
) -> Result<ProjectBootstrapStatus, AppError> {
    load_status(&PathBuf::from(project_path))
}

#[command]
pub async fn resume_project_bootstrap(
    project_path: String,
) -> Result<ProjectBootstrapStatus, AppError> {
    let project_path = PathBuf::from(project_path);
    guard_against_active_job(&project_path)?;

    let request = load_request(&project_path)?;
    let previous = load_status(&project_path)?;
    let retry_kinds = previous
        .failed_steps
        .iter()
        .filter_map(|step| BootstrapArtifactKind::from_step_name(step))
        .collect::<Vec<_>>();

    if retry_kinds.is_empty() {
        return Ok(previous);
    }

    let retry_status = ProjectBootstrapStatus::retry_from(
        &previous,
        uuid::Uuid::new_v4().to_string(),
        &retry_kinds,
    );
    save_status(&project_path, &retry_status)?;
    spawn_bootstrap_job(project_path, request, retry_kinds, retry_status.clone());

    Ok(retry_status)
}

fn spawn_bootstrap_job(
    project_path: PathBuf,
    request: PersistedBootstrapRequest,
    requested_kinds: Vec<BootstrapArtifactKind>,
    status: ProjectBootstrapStatus,
) {
    let generator = default_generator();
    tokio::spawn(async move {
        if let Err(error) = run_bootstrap_job(
            project_path.clone(),
            request,
            requested_kinds,
            status,
            generator,
        )
        .await
        {
            tracing::error!(
                target: "bootstrap::runner",
                project = %project_path.to_string_lossy(),
                error = %error,
                "project bootstrap task failed"
            );
        }
    });
}

fn load_project_metadata(project_path: &PathBuf) -> Result<ProjectMetadata, AppError> {
    let project_file = project_path.join(PROJECT_FILE);
    if !project_file.exists() {
        return Err(AppError::not_found(format!(
            "project file not found: {}",
            project_file.to_string_lossy()
        )));
    }

    let mut project: ProjectMetadata = read_json(&project_file)?;
    project.ensure_defaults();
    Ok(project)
}

fn guard_against_active_job(project_path: &PathBuf) -> Result<(), AppError> {
    match load_status(project_path) {
        Ok(status) if status.is_active() => Err(AppError {
            code: ErrorCode::Conflict,
            message: "bootstrap job already running for this project".to_string(),
            details: None,
            recoverable: Some(true),
        }),
        Ok(_) | Err(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::application::command_usecases::project::create_project;

    use super::{
        get_project_bootstrap_status, resume_project_bootstrap, start_project_bootstrap,
        StartProjectBootstrapInput,
    };
    use crate::application::command_usecases::bootstrap::types::ProjectBootstrapPhase;

    #[tokio::test]
    async fn start_project_bootstrap_spawns_background_job_and_persists_status() {
        let temp = tempfile::tempdir().expect("temp");
        let project_path = temp.path().join("novel");

        create_project(
            project_path.to_string_lossy().to_string(),
            "Novel".to_string(),
            "Tester".to_string(),
            None,
            None,
            Some("简介".to_string()),
            Some(90_000),
            Some(3),
            None,
            Some(3_000),
            None,
            None,
            None,
        )
        .await
        .expect("create");

        start_project_bootstrap(StartProjectBootstrapInput {
            project_path: project_path.to_string_lossy().to_string(),
            creation_brief: "主角必须在陌生城邦中做出选择".to_string(),
            description: Some("简介".to_string()),
            ..StartProjectBootstrapInput::default()
        })
        .await
        .expect("start bootstrap");

        let final_status =
            wait_for_terminal_status(project_path.to_string_lossy().to_string()).await;
        assert!(matches!(
            final_status.phase,
            ProjectBootstrapPhase::ReadyToWrite | ProjectBootstrapPhase::ReadyForReview
        ));
        assert!(final_status.failed_steps.is_empty());
    }

    #[tokio::test]
    async fn resume_without_failed_steps_returns_previous_status() {
        let temp = tempfile::tempdir().expect("temp");
        let project_path = temp.path().join("novel");

        create_project(
            project_path.to_string_lossy().to_string(),
            "Novel".to_string(),
            "Tester".to_string(),
            None,
            None,
            Some("简介".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("create");

        start_project_bootstrap(StartProjectBootstrapInput {
            project_path: project_path.to_string_lossy().to_string(),
            creation_brief: "简介".to_string(),
            ..StartProjectBootstrapInput::default()
        })
        .await
        .expect("start");

        let final_status =
            wait_for_terminal_status(project_path.to_string_lossy().to_string()).await;
        let resumed = resume_project_bootstrap(project_path.to_string_lossy().to_string())
            .await
            .expect("resume");

        assert_eq!(resumed.creation_job_id, final_status.creation_job_id);
        assert_eq!(resumed.phase, final_status.phase);
    }

    async fn wait_for_terminal_status(project_path: String) -> super::ProjectBootstrapStatus {
        for _ in 0..120 {
            let status = get_project_bootstrap_status(project_path.clone())
                .await
                .expect("status");
            if !status.is_active() {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        panic!("bootstrap job did not reach terminal state");
    }
}
