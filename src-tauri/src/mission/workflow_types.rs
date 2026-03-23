use serde::{Deserialize, Serialize};

use super::types::MissionState;

pub const WORKFLOW_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissionWorkflowKind {
    AdHoc,
    Macro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCreationReason {
    ExplicitMissionRequest,
    MacroWorkflow,
    ResumeRecovery,
    ReviewFixup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SummaryJobPolicy {
    ParentSessionSummary,
    ExplicitSummaryJob,
    NoSummaryJob,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Draft,
    Ready,
    Running,
    Blocked,
    WaitingUser,
    WaitingReview,
    WaitingKnowledgeDecision,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl WorkflowStatus {
    pub fn from_mission_state(state: &MissionState) -> Self {
        match state {
            MissionState::AwaitingInput => WorkflowStatus::Draft,
            MissionState::Initializing | MissionState::OrchestratorTurn => WorkflowStatus::Ready,
            MissionState::Running => WorkflowStatus::Running,
            MissionState::Blocked => WorkflowStatus::Blocked,
            MissionState::WaitingUser => WorkflowStatus::WaitingUser,
            MissionState::WaitingReview => WorkflowStatus::WaitingReview,
            MissionState::WaitingKnowledgeDecision => WorkflowStatus::WaitingKnowledgeDecision,
            MissionState::Paused => WorkflowStatus::Paused,
            MissionState::Completed => WorkflowStatus::Completed,
            MissionState::Failed => WorkflowStatus::Failed,
            MissionState::Cancelled => WorkflowStatus::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDoc {
    pub schema_version: i32,
    pub mission_id: String,
    pub workflow_kind: MissionWorkflowKind,
    pub creation_reason: WorkflowCreationReason,
    pub summary_job_policy: SummaryJobPolicy,
    pub status: WorkflowStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

impl WorkflowDoc {
    pub fn new(
        mission_id: String,
        workflow_kind: MissionWorkflowKind,
        creation_reason: WorkflowCreationReason,
        summary_job_policy: SummaryJobPolicy,
        status: WorkflowStatus,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: WORKFLOW_SCHEMA_VERSION,
            mission_id,
            workflow_kind,
            creation_reason,
            summary_job_policy,
            status,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_status_maps_from_mission_state() {
        assert_eq!(
            WorkflowStatus::from_mission_state(&MissionState::AwaitingInput),
            WorkflowStatus::Draft
        );
        assert_eq!(
            WorkflowStatus::from_mission_state(&MissionState::OrchestratorTurn),
            WorkflowStatus::Ready
        );
        assert_eq!(
            WorkflowStatus::from_mission_state(&MissionState::WaitingReview),
            WorkflowStatus::WaitingReview
        );
        assert_eq!(
            WorkflowStatus::from_mission_state(&MissionState::Cancelled),
            WorkflowStatus::Cancelled
        );
    }
}
