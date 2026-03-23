use std::path::PathBuf;
use std::sync::Arc;

use crate::knowledge::bootstrap::writeback::write_bootstrap_outputs;
use crate::llm::bootstrap::{BootstrapArtifactKind, BootstrapGenerationResult, BootstrapGenerator};
use crate::models::AppError;

use super::persistence::{save_status, PersistedBootstrapRequest};
use super::types::{
    ordered_unique_steps, ProjectBootstrapArtifact, ProjectBootstrapPhase, ProjectBootstrapStatus,
};

pub async fn run_bootstrap_job(
    project_path: PathBuf,
    request: PersistedBootstrapRequest,
    requested_kinds: Vec<BootstrapArtifactKind>,
    mut status: ProjectBootstrapStatus,
    generator: Arc<dyn BootstrapGenerator>,
) -> Result<ProjectBootstrapStatus, AppError> {
    status.set_phase(
        ProjectBootstrapPhase::AssemblingPrompt,
        10,
        "assemble_bootstrap_prompt",
    );
    save_status(&project_path, &status)?;

    status.set_phase(
        ProjectBootstrapPhase::LlmGenerating,
        35,
        "generate_bootstrap_artifacts",
    );
    save_status(&project_path, &status)?;

    let generation = match generator
        .generate(request.prompt_input.clone(), requested_kinds.clone())
        .await
    {
        Ok(result) => result,
        Err(error) => {
            let failed_steps = requested_kinds
                .iter()
                .map(|kind| kind.as_str().to_string())
                .collect::<Vec<_>>();
            let phase = if status.generated_artifacts.is_empty() {
                ProjectBootstrapPhase::Failed
            } else {
                ProjectBootstrapPhase::PartiallyGenerated
            };
            status.set_terminal(
                phase,
                100,
                status.completed_steps.clone(),
                failed_steps,
                status.generated_artifacts.clone(),
                "resume_bootstrap",
                Some(error.message),
            );
            save_status(&project_path, &status)?;
            return Ok(status);
        }
    };

    status.set_phase(
        ProjectBootstrapPhase::WritingArtifacts,
        70,
        "write_bootstrap_artifacts",
    );
    save_status(&project_path, &status)?;

    let mut writeback = write_bootstrap_outputs(
        &project_path,
        &generation,
        &requested_kinds,
        &status.creation_job_id,
    );

    let existing_artifacts = status.generated_artifacts.clone();
    let merged_artifacts =
        merge_artifacts(existing_artifacts, writeback.artifacts.drain(..).collect());
    let failed_steps = merge_failed_steps(&generation, &writeback.failed_kinds);
    let completed_steps = merged_artifacts
        .iter()
        .map(|artifact| artifact.kind.clone())
        .collect::<Vec<_>>();

    let final_phase =
        determine_terminal_phase(&request.requested_kinds, &merged_artifacts, &failed_steps);
    let progress = if matches!(final_phase, ProjectBootstrapPhase::ReadyToWrite) {
        100
    } else {
        90
    };
    let next_action = if failed_steps.is_empty() {
        writeback.recommended_next_action
    } else {
        "resume_bootstrap".to_string()
    };
    status.set_terminal(
        final_phase,
        progress,
        ordered_unique_steps(completed_steps),
        failed_steps.clone(),
        merged_artifacts,
        next_action,
        build_error_message(&generation, &failed_steps),
    );

    save_status(&project_path, &status)?;
    Ok(status)
}

fn determine_terminal_phase(
    requested_kinds: &[BootstrapArtifactKind],
    generated_artifacts: &[ProjectBootstrapArtifact],
    failed_steps: &[String],
) -> ProjectBootstrapPhase {
    if generated_artifacts.is_empty() && !failed_steps.is_empty() {
        return ProjectBootstrapPhase::Failed;
    }

    let generated_kinds = generated_artifacts
        .iter()
        .map(|artifact| artifact.kind.as_str())
        .collect::<std::collections::HashSet<_>>();
    let all_requested_present = requested_kinds
        .iter()
        .all(|kind| generated_kinds.contains(kind.as_str()));

    if failed_steps.is_empty() && all_requested_present {
        ProjectBootstrapPhase::ReadyToWrite
    } else if failed_steps.is_empty() {
        ProjectBootstrapPhase::ReadyForReview
    } else if generated_artifacts.is_empty() {
        ProjectBootstrapPhase::Failed
    } else {
        ProjectBootstrapPhase::PartiallyGenerated
    }
}

fn merge_artifacts(
    existing: Vec<ProjectBootstrapArtifact>,
    new_items: Vec<ProjectBootstrapArtifact>,
) -> Vec<ProjectBootstrapArtifact> {
    let mut merged = std::collections::BTreeMap::new();
    for artifact in existing.into_iter().chain(new_items.into_iter()) {
        merged.insert(artifact.kind.clone(), artifact);
    }
    let mut values = merged.into_values().collect::<Vec<_>>();
    values.sort_by_key(|artifact| {
        BootstrapArtifactKind::from_step_name(&artifact.kind)
            .map(|kind| kind.order_key())
            .unwrap_or(usize::MAX)
    });
    values
}

fn merge_failed_steps(
    generation: &BootstrapGenerationResult,
    writeback_failures: &[BootstrapArtifactKind],
) -> Vec<String> {
    let failures = generation
        .failures
        .iter()
        .map(|failure| failure.kind.as_str().to_string())
        .chain(
            writeback_failures
                .iter()
                .map(|kind| kind.as_str().to_string()),
        );
    ordered_unique_steps(failures)
}

fn build_error_message(
    generation: &BootstrapGenerationResult,
    failed_steps: &[String],
) -> Option<String> {
    if failed_steps.is_empty() {
        return None;
    }

    let messages = generation
        .failures
        .iter()
        .map(|failure| format!("{}: {}", failure.kind.as_str(), failure.message))
        .collect::<Vec<_>>();
    if messages.is_empty() {
        Some("bootstrap completed with partial failures".to_string())
    } else {
        Some(messages.join(" | "))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::application::command_usecases::bootstrap::persistence::{
        save_request, PersistedBootstrapRequest,
    };
    use crate::application::command_usecases::bootstrap::types::{
        ProjectBootstrapPhase, ProjectBootstrapStatus,
    };
    use crate::application::command_usecases::project::create_project;
    use crate::llm::bootstrap::{
        BootstrapArtifactFailure, BootstrapArtifactKind, BootstrapChapterPlan,
        BootstrapCreativePayload, BootstrapGenerationResult, BootstrapGenerator,
        BootstrapPromptInput, BootstrapVolumePlan,
    };
    use crate::models::{AppError, ProjectMetadata};
    use crate::services::read_json;

    use super::run_bootstrap_job;

    struct FakeGenerator {
        requested: Arc<Mutex<Vec<Vec<BootstrapArtifactKind>>>>,
        fail_once: Arc<Mutex<Option<BootstrapArtifactKind>>>,
    }

    #[async_trait]
    impl BootstrapGenerator for FakeGenerator {
        fn name(&self) -> &'static str {
            "fake"
        }

        async fn generate(
            &self,
            input: BootstrapPromptInput,
            requested_kinds: Vec<BootstrapArtifactKind>,
        ) -> Result<BootstrapGenerationResult, AppError> {
            self.requested
                .lock()
                .expect("requested")
                .push(requested_kinds.clone());

            let fail_kind = self.fail_once.lock().expect("fail_once").take();
            let payload = BootstrapCreativePayload {
                story_blueprint: format!("故事总纲：{}", input.project_name),
                theme_notes: "主题注记".to_string(),
                protagonist_seed: "主角种子".to_string(),
                counterpart_seed: "对手种子".to_string(),
                world_summary: "世界摘要".to_string(),
                main_plotline: "主线推进".to_string(),
                volumes: vec![BootstrapVolumePlan {
                    title: "卷一".to_string(),
                    summary: "卷一摘要".to_string(),
                    dramatic_goal: "引爆主线".to_string(),
                    target_words: 12_000,
                    chapters: vec![BootstrapChapterPlan {
                        title: "第一章".to_string(),
                        summary: "第一章摘要".to_string(),
                        plot_goal: "建立冲突".to_string(),
                        emotional_goal: "制造不安".to_string(),
                        target_words: 3_000,
                    }],
                }],
                recommended_next_action: "start_chapter_one".to_string(),
            };

            let mut result = payload.materialize(&requested_kinds, self.name());
            if let Some(kind) = fail_kind {
                result.artifacts.retain(|artifact| artifact.kind != kind);
                result.failures.push(BootstrapArtifactFailure {
                    kind,
                    message: "simulated failure".to_string(),
                });
            }

            Ok(result)
        }
    }

    #[tokio::test]
    async fn run_bootstrap_job_generates_artifacts_and_structure() {
        let temp = tempfile::tempdir().expect("temp");
        let project_path = temp.path().join("novel");
        create_project(
            project_path.to_string_lossy().to_string(),
            "Novel".to_string(),
            "Tester".to_string(),
            Some(vec!["fantasy".to_string()]),
            None,
            Some("一个关于命运反转的故事".to_string()),
            Some(120_000),
            Some(3),
            None,
            Some(3_500),
            Some("third_limited".to_string()),
            Some(vec!["克制".to_string(), "悬疑".to_string()]),
            Some("general".to_string()),
        )
        .await
        .expect("create project");

        let project = load_project(&project_path);
        let prompt_input = BootstrapPromptInput::from_project(
            &project,
            "主角在废墟都市中发现失控装置。".to_string(),
            None,
            None,
            None,
            Some("结局要留下代价".to_string()),
        );
        let request = PersistedBootstrapRequest::new(
            project.project_id.clone(),
            prompt_input,
            BootstrapArtifactKind::all().to_vec(),
        );
        save_request(&project_path, &request, &project).expect("save request");
        let status = ProjectBootstrapStatus::pending(
            project.project_id.clone(),
            uuid::Uuid::new_v4().to_string(),
        );

        let requested = Arc::new(Mutex::new(Vec::new()));
        let generator = Arc::new(FakeGenerator {
            requested,
            fail_once: Arc::new(Mutex::new(None)),
        });

        let final_status = run_bootstrap_job(
            project_path.clone(),
            request,
            BootstrapArtifactKind::all().to_vec(),
            status,
            generator,
        )
        .await
        .expect("run bootstrap");

        assert_eq!(final_status.phase, ProjectBootstrapPhase::ReadyToWrite);
        assert!(project_path
            .join(".magic_novel")
            .join("planning")
            .join("story_blueprint.md")
            .exists());
        assert!(project_path
            .join(".magic_novel")
            .join("index")
            .join("object_index.json")
            .exists());
        assert!(project_path
            .join("manuscripts")
            .join("bootstrap-v01")
            .join("volume.json")
            .exists());
    }

    #[tokio::test]
    async fn run_bootstrap_job_marks_partial_and_resume_only_failed_steps() {
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
        .expect("create project");

        let project = load_project(&project_path);
        let prompt_input = BootstrapPromptInput::from_project(
            &project,
            "简介".to_string(),
            None,
            None,
            None,
            None,
        );
        let request = PersistedBootstrapRequest::new(
            project.project_id.clone(),
            prompt_input,
            BootstrapArtifactKind::all().to_vec(),
        );
        save_request(&project_path, &request, &project).expect("save request");
        let status = ProjectBootstrapStatus::pending(
            project.project_id.clone(),
            uuid::Uuid::new_v4().to_string(),
        );

        let requested = Arc::new(Mutex::new(Vec::new()));
        let fail_once = Arc::new(Mutex::new(Some(BootstrapArtifactKind::ThemeNotes)));
        let generator = Arc::new(FakeGenerator {
            requested: requested.clone(),
            fail_once,
        });

        let partial = run_bootstrap_job(
            project_path.clone(),
            request.clone(),
            BootstrapArtifactKind::all().to_vec(),
            status,
            generator.clone(),
        )
        .await
        .expect("partial");

        assert_eq!(partial.phase, ProjectBootstrapPhase::PartiallyGenerated);
        assert_eq!(partial.failed_steps, vec!["theme_notes".to_string()]);

        let retry_kinds = partial
            .failed_steps
            .iter()
            .filter_map(|step| BootstrapArtifactKind::from_step_name(step))
            .collect::<Vec<_>>();
        let retry_status = ProjectBootstrapStatus::retry_from(
            &partial,
            uuid::Uuid::new_v4().to_string(),
            &retry_kinds,
        );
        let final_status = run_bootstrap_job(
            project_path.clone(),
            request,
            retry_kinds,
            retry_status,
            generator,
        )
        .await
        .expect("resume");

        assert_eq!(final_status.phase, ProjectBootstrapPhase::ReadyToWrite);
        let requested_batches = requested.lock().expect("requested");
        assert_eq!(requested_batches.len(), 2);
        assert_eq!(
            requested_batches[1],
            vec![BootstrapArtifactKind::ThemeNotes]
        );
    }

    fn load_project(project_path: &Path) -> ProjectMetadata {
        read_json(&project_path.join("project.json")).expect("project")
    }
}
