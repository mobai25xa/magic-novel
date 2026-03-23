//! Shared contract seed for job snapshots, job events, and resource locks.
//!
//! Batch 0 freezes the names and baseline semantics without forcing an
//! immediate rewrite of the existing mission status pipeline.

use serde::{Deserialize, Serialize};

use super::blockers::{WorkflowBlocker, WorkflowBlockerKind};
use super::delegate_types::DelegateResult;
use super::events::MissionEventEnvelope;
use super::workflow_types::{MissionWorkflowKind, WorkflowDoc, WorkflowStatus};

pub const JOB_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    MissionAdHoc,
    MacroWorkflow,
    DelegateBatch,
    Unknown,
}

impl Default for JobKind {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<MissionWorkflowKind> for JobKind {
    fn from(value: MissionWorkflowKind) -> Self {
        match value {
            MissionWorkflowKind::AdHoc => Self::MissionAdHoc,
            MissionWorkflowKind::Macro => Self::MacroWorkflow,
        }
    }
}

impl From<&MissionWorkflowKind> for JobKind {
    fn from(value: &MissionWorkflowKind) -> Self {
        value.clone().into()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
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

impl Default for JobStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl From<WorkflowStatus> for JobStatus {
    fn from(value: WorkflowStatus) -> Self {
        (&value).into()
    }
}

impl From<&WorkflowStatus> for JobStatus {
    fn from(value: &WorkflowStatus) -> Self {
        match value {
            WorkflowStatus::Draft => Self::Draft,
            WorkflowStatus::Ready => Self::Ready,
            WorkflowStatus::Running => Self::Running,
            WorkflowStatus::Blocked => Self::Blocked,
            WorkflowStatus::WaitingUser => Self::WaitingUser,
            WorkflowStatus::WaitingReview => Self::WaitingReview,
            WorkflowStatus::WaitingKnowledgeDecision => Self::WaitingKnowledgeDecision,
            WorkflowStatus::Paused => Self::Paused,
            WorkflowStatus::Completed => Self::Completed,
            WorkflowStatus::Failed => Self::Failed,
            WorkflowStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceLockKind {
    File,
    Chapter,
    Canon,
    Review,
    ExternalDependency,
}

impl Default for ResourceLockKind {
    fn default() -> Self {
        Self::File
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceLockMode {
    Shared,
    Exclusive,
}

impl Default for ResourceLockMode {
    fn default() -> Self {
        Self::Exclusive
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ResourceLock {
    pub lock_id: String,
    pub lock_kind: ResourceLockKind,
    pub scope: String,
    pub mode: ResourceLockMode,
}

impl Default for ResourceLock {
    fn default() -> Self {
        Self {
            lock_id: String::new(),
            lock_kind: ResourceLockKind::default(),
            scope: String::new(),
            mode: ResourceLockMode::default(),
        }
    }
}

impl ResourceLock {
    pub fn normalized(mut self) -> Self {
        self.lock_id = self.lock_id.trim().to_string();
        self.scope = self.scope.trim().to_string();
        self
    }
}

pub type JobBlocker = WorkflowBlocker;
pub type JobBlockerKind = WorkflowBlockerKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JobSnapshot {
    pub schema_version: i32,
    pub job_id: String,
    pub job_kind: JobKind,
    pub status: JobStatus,
    pub blockers: Vec<JobBlocker>,
    pub ready_tasks: Vec<String>,
    pub running_tasks: Vec<String>,
    pub completed_tasks: Vec<String>,
    pub failed_tasks: Vec<String>,
    pub task_results: Vec<DelegateResult>,
    pub updated_at: i64,
}

impl Default for JobSnapshot {
    fn default() -> Self {
        Self::new(String::new(), JobKind::Unknown, JobStatus::Draft)
    }
}

impl JobSnapshot {
    pub fn new(job_id: impl Into<String>, job_kind: JobKind, status: JobStatus) -> Self {
        Self {
            schema_version: JOB_SCHEMA_VERSION,
            job_id: job_id.into().trim().to_string(),
            job_kind,
            status,
            blockers: Vec::new(),
            ready_tasks: Vec::new(),
            running_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            failed_tasks: Vec::new(),
            task_results: Vec::new(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn from_workflow(workflow: &WorkflowDoc) -> Self {
        Self::new(
            workflow.mission_id.clone(),
            JobKind::from(&workflow.workflow_kind),
            JobStatus::from(&workflow.status),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JobEvent {
    pub schema_version: i32,
    pub event_type: String,
    pub job_id: String,
    pub task_id: String,
    pub payload: serde_json::Value,
    pub ts: i64,
}

impl Default for JobEvent {
    fn default() -> Self {
        Self {
            schema_version: JOB_SCHEMA_VERSION,
            event_type: String::new(),
            job_id: String::new(),
            task_id: String::new(),
            payload: serde_json::Value::Null,
            ts: 0,
        }
    }
}

impl JobEvent {
    pub fn new(
        job_id: impl Into<String>,
        task_id: impl Into<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            schema_version: JOB_SCHEMA_VERSION,
            event_type: event_type.into().trim().to_string(),
            job_id: job_id.into().trim().to_string(),
            task_id: task_id.into().trim().to_string(),
            payload,
            ts: chrono::Utc::now().timestamp_millis(),
        }
    }
}

impl From<&MissionEventEnvelope> for JobEvent {
    fn from(value: &MissionEventEnvelope) -> Self {
        let task_id = value
            .payload
            .get("task_id")
            .and_then(|raw| raw.as_str())
            .or_else(|| value.payload.get("feature_id").and_then(|raw| raw.as_str()))
            .unwrap_or_default()
            .trim()
            .to_string();

        Self {
            schema_version: value.schema_version,
            event_type: value.event_type.clone(),
            job_id: value.mission_id.clone(),
            task_id,
            payload: value.payload.clone(),
            ts: value.ts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn job_snapshot_maps_from_workflow_doc() {
        let workflow = WorkflowDoc::new(
            "mis_1".to_string(),
            MissionWorkflowKind::Macro,
            super::super::workflow_types::WorkflowCreationReason::MacroWorkflow,
            super::super::workflow_types::SummaryJobPolicy::NoSummaryJob,
            WorkflowStatus::Running,
        );

        let snapshot = JobSnapshot::from_workflow(&workflow);

        assert_eq!(snapshot.job_id, "mis_1");
        assert_eq!(snapshot.job_kind, JobKind::MacroWorkflow);
        assert_eq!(snapshot.status, JobStatus::Running);
        assert_eq!(snapshot.schema_version, JOB_SCHEMA_VERSION);
    }

    #[test]
    fn job_event_bridges_existing_mission_event_envelope() {
        let envelope = MissionEventEnvelope {
            schema_version: 1,
            event_id: "mevt_1".to_string(),
            ts: 123,
            mission_id: "mis_1".to_string(),
            event_type: "WORKER_COMPLETED".to_string(),
            payload: json!({
                "feature_id": "feat_1",
                "summary": "done"
            }),
        };

        let event = JobEvent::from(&envelope);

        assert_eq!(event.job_id, "mis_1");
        assert_eq!(event.task_id, "feat_1");
        assert_eq!(event.event_type, "WORKER_COMPLETED");
    }

    #[test]
    fn resource_lock_normalizes_strings() {
        let lock = ResourceLock {
            lock_id: " lock_1 ".to_string(),
            lock_kind: ResourceLockKind::Chapter,
            scope: " chapter:1 ".to_string(),
            mode: ResourceLockMode::Exclusive,
        }
        .normalized();

        assert_eq!(lock.lock_id, "lock_1");
        assert_eq!(lock.scope, "chapter:1");
    }
}
