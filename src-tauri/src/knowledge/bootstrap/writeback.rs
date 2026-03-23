use std::path::Path;

use crate::application::command_usecases::bootstrap::ProjectBootstrapArtifact;
use crate::llm::bootstrap::{BootstrapArtifactKind, BootstrapGenerationResult, GeneratedArtifact};
use crate::models::AppError;
use crate::services::{ensure_dir, write_file};

use super::index::upsert_index_entries;
use super::structure::sync_bootstrap_structure;

#[derive(Debug, Clone)]
pub struct BootstrapWrittenArtifact {
    pub kind: BootstrapArtifactKind,
    pub path: String,
    pub status: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct BootstrapWritebackOutcome {
    pub artifacts: Vec<ProjectBootstrapArtifact>,
    pub failed_kinds: Vec<BootstrapArtifactKind>,
    pub recommended_next_action: String,
}

pub fn write_bootstrap_outputs(
    project_path: &Path,
    generation: &BootstrapGenerationResult,
    requested_kinds: &[BootstrapArtifactKind],
    source_job_id: &str,
) -> BootstrapWritebackOutcome {
    let mut written_rows = Vec::new();
    let mut artifact_rows = Vec::new();
    let mut failed_kinds = generation
        .failures
        .iter()
        .map(|failure| failure.kind)
        .collect::<Vec<_>>();

    for artifact in generation
        .artifacts
        .iter()
        .filter(|artifact| requested_kinds.iter().any(|kind| kind == &artifact.kind))
    {
        match write_single_artifact(project_path, artifact) {
            Ok(written) => {
                artifact_rows.push(written.clone());
                written_rows.push(ProjectBootstrapArtifact::new(
                    written.kind.as_str(),
                    written.path.clone(),
                    written.status.clone(),
                    written.title.clone(),
                    written.summary.clone(),
                    written.updated_at,
                ));
            }
            Err(_) => failed_kinds.push(artifact.kind),
        }
    }

    if !artifact_rows.is_empty()
        && upsert_index_entries(project_path, &artifact_rows, source_job_id).is_err()
    {
        failed_kinds.extend(artifact_rows.iter().map(|artifact| artifact.kind));
    }

    let needs_structure = requested_kinds.iter().any(|kind| {
        matches!(
            kind,
            BootstrapArtifactKind::VolumePlan | BootstrapArtifactKind::ChapterBacklog
        )
    });
    if needs_structure && !generation.volumes.is_empty() {
        if sync_bootstrap_structure(project_path, &generation.volumes).is_err() {
            failed_kinds.push(BootstrapArtifactKind::VolumePlan);
            failed_kinds.push(BootstrapArtifactKind::ChapterBacklog);
        }
    }

    failed_kinds.sort_by_key(BootstrapArtifactKind::order_key);
    failed_kinds.dedup();

    BootstrapWritebackOutcome {
        artifacts: written_rows,
        failed_kinds,
        recommended_next_action: generation.recommended_next_action.clone(),
    }
}

fn write_single_artifact(
    project_path: &Path,
    artifact: &GeneratedArtifact,
) -> Result<BootstrapWrittenArtifact, AppError> {
    let relative_path = artifact.kind.knowledge_path();
    let full_path = project_path.join(relative_path);
    if let Some(parent) = full_path.parent() {
        ensure_dir(parent)?;
    }
    write_file(&full_path, &artifact.content)?;

    Ok(BootstrapWrittenArtifact {
        kind: artifact.kind,
        path: relative_path.to_string(),
        status: artifact.status.clone(),
        title: Some(artifact.title.clone()),
        summary: artifact.summary.clone(),
        updated_at: chrono::Utc::now().timestamp_millis(),
    })
}
