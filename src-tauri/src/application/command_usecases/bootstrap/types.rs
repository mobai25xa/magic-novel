use serde::{Deserialize, Serialize};

use crate::llm::bootstrap::BootstrapArtifactKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectBootstrapPhase {
    Pending,
    AssemblingPrompt,
    LlmGenerating,
    WritingArtifacts,
    PartiallyGenerated,
    ReadyForReview,
    ReadyToWrite,
    Failed,
}

impl ProjectBootstrapPhase {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Pending | Self::AssemblingPrompt | Self::LlmGenerating | Self::WritingArtifacts
        )
    }

    pub fn bootstrap_state_label(&self) -> &'static str {
        match self {
            Self::Pending
            | Self::AssemblingPrompt
            | Self::LlmGenerating
            | Self::WritingArtifacts => "bootstrap_running",
            Self::PartiallyGenerated => "partially_generated",
            Self::ReadyForReview => "ready_for_review",
            Self::ReadyToWrite => "ready_to_write",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectBootstrapArtifact {
    pub kind: String,
    pub path: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub updated_at: i64,
}

impl ProjectBootstrapArtifact {
    pub fn new(
        kind: impl Into<String>,
        path: impl Into<String>,
        status: impl Into<String>,
        title: Option<String>,
        summary: Option<String>,
        updated_at: i64,
    ) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
            status: status.into(),
            title,
            summary,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBootstrapStatus {
    pub project_id: String,
    pub creation_job_id: String,
    pub phase: ProjectBootstrapPhase,
    pub progress: i32,
    pub bootstrap_state: String,
    #[serde(default)]
    pub completed_steps: Vec<String>,
    #[serde(default)]
    pub failed_steps: Vec<String>,
    #[serde(default)]
    pub generated_artifacts: Vec<ProjectBootstrapArtifact>,
    pub recommended_next_action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub started_at: i64,
    pub updated_at: i64,
}

impl ProjectBootstrapStatus {
    pub fn pending(project_id: String, creation_job_id: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            project_id,
            creation_job_id,
            phase: ProjectBootstrapPhase::Pending,
            progress: 0,
            bootstrap_state: ProjectBootstrapPhase::Pending
                .bootstrap_state_label()
                .to_string(),
            completed_steps: Vec::new(),
            failed_steps: Vec::new(),
            generated_artifacts: Vec::new(),
            recommended_next_action: "wait_for_bootstrap".to_string(),
            error_message: None,
            started_at: now,
            updated_at: now,
        }
    }

    pub fn retry_from(
        previous: &Self,
        creation_job_id: String,
        retry_kinds: &[BootstrapArtifactKind],
    ) -> Self {
        let retry_steps: Vec<String> = retry_kinds
            .iter()
            .map(|kind| kind.as_str().to_string())
            .collect();
        let mut artifacts = previous.generated_artifacts.clone();
        artifacts.retain(|artifact| !retry_steps.iter().any(|step| step == &artifact.kind));

        let completed_steps: Vec<String> = previous
            .completed_steps
            .iter()
            .filter(|step| !retry_steps.iter().any(|retry| retry == *step))
            .cloned()
            .collect();

        let mut status = Self::pending(previous.project_id.clone(), creation_job_id);
        status.generated_artifacts = sort_artifacts(artifacts);
        status.completed_steps = ordered_unique_steps(completed_steps);
        status.recommended_next_action = "wait_for_bootstrap".to_string();
        status
    }

    pub fn is_active(&self) -> bool {
        self.phase.is_active()
    }

    pub fn set_phase(
        &mut self,
        phase: ProjectBootstrapPhase,
        progress: i32,
        recommended_next_action: impl Into<String>,
    ) {
        self.phase = phase;
        self.progress = progress.clamp(0, 100);
        self.bootstrap_state = self.phase.bootstrap_state_label().to_string();
        self.recommended_next_action = recommended_next_action.into();
        self.updated_at = chrono::Utc::now().timestamp_millis();
        self.error_message = None;
    }

    pub fn set_terminal(
        &mut self,
        phase: ProjectBootstrapPhase,
        progress: i32,
        completed_steps: Vec<String>,
        failed_steps: Vec<String>,
        generated_artifacts: Vec<ProjectBootstrapArtifact>,
        recommended_next_action: impl Into<String>,
        error_message: Option<String>,
    ) {
        self.phase = phase;
        self.progress = progress.clamp(0, 100);
        self.bootstrap_state = self.phase.bootstrap_state_label().to_string();
        self.completed_steps = ordered_unique_steps(completed_steps);
        self.failed_steps = ordered_unique_steps(failed_steps);
        self.generated_artifacts = sort_artifacts(generated_artifacts);
        self.recommended_next_action = recommended_next_action.into();
        self.error_message = error_message;
        self.updated_at = chrono::Utc::now().timestamp_millis();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StartProjectBootstrapInput {
    pub project_path: String,
    pub creation_brief: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub target_total_words: Option<i32>,
    #[serde(default)]
    pub planned_volumes: Option<i32>,
    #[serde(default)]
    pub target_words_per_volume: Option<i32>,
    #[serde(default)]
    pub target_words_per_chapter: Option<i32>,
    #[serde(default)]
    pub narrative_pov: Option<String>,
    #[serde(default)]
    pub tone: Option<Vec<String>>,
    #[serde(default)]
    pub audience: Option<String>,
    #[serde(default)]
    pub protagonist_seed: Option<String>,
    #[serde(default)]
    pub counterpart_seed: Option<String>,
    #[serde(default)]
    pub world_seed: Option<String>,
    #[serde(default)]
    pub ending_direction: Option<String>,
}

impl StartProjectBootstrapInput {
    pub fn normalize(&mut self) {
        self.project_path = self.project_path.trim().to_string();
        self.creation_brief = self.creation_brief.trim().to_string();
        self.description = normalize_optional_string(self.description.take());
        self.narrative_pov = normalize_optional_string(self.narrative_pov.take());
        self.audience = normalize_optional_string(self.audience.take());
        self.protagonist_seed = normalize_optional_string(self.protagonist_seed.take());
        self.counterpart_seed = normalize_optional_string(self.counterpart_seed.take());
        self.world_seed = normalize_optional_string(self.world_seed.take());
        self.ending_direction = normalize_optional_string(self.ending_direction.take());
        self.tone = self
            .tone
            .take()
            .map(normalize_labels)
            .filter(|items| !items.is_empty());
    }
}

pub fn ordered_unique_steps<I>(steps: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut out = Vec::new();
    for step in steps {
        let step = step.trim();
        if step.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == step) {
            out.push(step.to_string());
        }
    }
    out.sort_by_key(|step| artifact_order_key(step));
    out
}

fn sort_artifacts(mut artifacts: Vec<ProjectBootstrapArtifact>) -> Vec<ProjectBootstrapArtifact> {
    artifacts.sort_by_key(|artifact| artifact_order_key(&artifact.kind));
    artifacts
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_labels(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn artifact_order_key(kind: &str) -> usize {
    BootstrapArtifactKind::from_step_name(kind)
        .map(|item| item.order_key())
        .unwrap_or(usize::MAX)
}
