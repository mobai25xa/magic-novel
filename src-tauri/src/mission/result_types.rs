use serde::{Deserialize, Serialize};

use crate::agent_engine::types::{StopReason, UsageInfo};

use super::types::HandoffEntry;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskResultStatus {
    Completed,
    Failed,
    Cancelled,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStopReason {
    Success,
    Error,
    Cancelled,
    Limit,
    WaitingConfirmation,
    WaitingAskuser,
    Blocked,
    Unknown,
}

impl From<StopReason> for TaskStopReason {
    fn from(value: StopReason) -> Self {
        (&value).into()
    }
}

impl From<&StopReason> for TaskStopReason {
    fn from(value: &StopReason) -> Self {
        match value {
            StopReason::Success => Self::Success,
            StopReason::Cancel => Self::Cancelled,
            StopReason::Error => Self::Error,
            StopReason::Limit => Self::Limit,
            StopReason::WaitingConfirmation => Self::WaitingConfirmation,
            StopReason::WaitingAskuser => Self::WaitingAskuser,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangedPathKind {
    Created,
    Modified,
    Deleted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedPath {
    pub path: String,
    #[serde(default)]
    pub change_kind: ChangedPathKind,
}

impl Default for ChangedPathKind {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRef {
    pub kind: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceItem {
    pub kind: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenIssue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUsage {
    pub rounds_executed: u32,
    pub total_tool_calls: u32,
    pub latency_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentTaskResult {
    pub task_id: String,
    pub actor_id: String,
    pub goal: String,
    pub status: TaskResultStatus,
    pub stop_reason: TaskStopReason,
    pub result_summary: String,
    pub changed_paths: Vec<ChangedPath>,
    pub artifacts: Vec<ArtifactRef>,
    pub evidence: Vec<EvidenceItem>,
    pub open_issues: Vec<OpenIssue>,
    pub next_actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TaskUsage>,
}

impl Default for AgentTaskResult {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            actor_id: String::new(),
            goal: String::new(),
            status: TaskResultStatus::Completed,
            stop_reason: TaskStopReason::Success,
            result_summary: String::new(),
            changed_paths: Vec::new(),
            artifacts: Vec::new(),
            evidence: Vec::new(),
            open_issues: Vec::new(),
            next_actions: Vec::new(),
            usage: None,
        }
    }
}

impl AgentTaskResult {
    pub fn is_ok(&self) -> bool {
        matches!(self.status, TaskResultStatus::Completed)
    }

    pub fn normalized_summary(&self) -> String {
        if self.result_summary.trim().is_empty() {
            match self.status {
                TaskResultStatus::Completed => "task completed".to_string(),
                TaskResultStatus::Failed => "task failed".to_string(),
                TaskResultStatus::Cancelled => "task cancelled".to_string(),
                TaskResultStatus::Blocked => "task blocked".to_string(),
            }
        } else {
            self.result_summary.trim().to_string()
        }
    }

    pub fn to_handoff_entry(&self, feature_id: &str, worker_id: &str) -> HandoffEntry {
        let summary = self.normalized_summary();

        let artifacts = if self.artifacts.is_empty() {
            self.changed_paths
                .iter()
                .map(|path| path.path.clone())
                .filter(|path| !path.trim().is_empty())
                .collect::<Vec<_>>()
        } else {
            self.artifacts
                .iter()
                .map(|artifact| artifact.value.clone())
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
        };

        let issues = self
            .open_issues
            .iter()
            .map(|issue| issue.summary.clone())
            .filter(|summary| !summary.trim().is_empty())
            .collect::<Vec<_>>();

        HandoffEntry {
            feature_id: feature_id.to_string(),
            worker_id: worker_id.to_string(),
            ok: self.is_ok(),
            summary,
            commands_run: Vec::new(),
            artifacts,
            issues,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_task_result_roundtrip() {
        let result = AgentTaskResult {
            task_id: "feat-1".to_string(),
            actor_id: "wk-1".to_string(),
            goal: "Write chapter draft".to_string(),
            status: TaskResultStatus::Completed,
            stop_reason: TaskStopReason::Success,
            result_summary: "chapter draft updated".to_string(),
            changed_paths: vec![ChangedPath {
                path: "manuscripts/ch1.json".to_string(),
                change_kind: ChangedPathKind::Modified,
            }],
            artifacts: vec![ArtifactRef {
                kind: "chapter".to_string(),
                value: "manuscripts/ch1.json".to_string(),
                description: Some("updated chapter".to_string()),
            }],
            evidence: vec![EvidenceItem {
                kind: "assistant_summary".to_string(),
                summary: "Updated scene beats".to_string(),
                value: None,
            }],
            open_issues: vec![OpenIssue {
                code: Some("WARN_FACTS".to_string()),
                summary: "fact check still recommended".to_string(),
                blocking: false,
            }],
            next_actions: vec!["run review".to_string()],
            usage: Some(TaskUsage {
                rounds_executed: 2,
                total_tool_calls: 3,
                latency_ms: 1234,
                llm_usage: None,
            }),
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: AgentTaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.task_id, "feat-1");
        assert_eq!(parsed.changed_paths.len(), 1);
        assert_eq!(parsed.open_issues[0].code.as_deref(), Some("WARN_FACTS"));
    }

    #[test]
    fn handoff_mapping_preserves_failure_summary_and_issue() {
        let result = AgentTaskResult {
            task_id: "feat-2".to_string(),
            actor_id: "wk-2".to_string(),
            goal: "Review chapter".to_string(),
            status: TaskResultStatus::Failed,
            stop_reason: TaskStopReason::Error,
            result_summary: "review failed".to_string(),
            open_issues: vec![OpenIssue {
                code: Some("E_REVIEW".to_string()),
                summary: "review service unavailable".to_string(),
                blocking: true,
            }],
            ..AgentTaskResult::default()
        };

        let handoff = result.to_handoff_entry("feat-2", "wk-2");
        assert!(!handoff.ok);
        assert_eq!(handoff.summary, "review failed");
        assert_eq!(
            handoff.issues,
            vec!["review service unavailable".to_string()]
        );
    }

    #[test]
    fn normalized_summary_falls_back_to_status_label() {
        let result = AgentTaskResult {
            status: TaskResultStatus::Blocked,
            ..AgentTaskResult::default()
        };

        assert_eq!(result.normalized_summary(), "task blocked");
    }
}
