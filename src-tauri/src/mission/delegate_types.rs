//! Shared contract seed for delegate requests and structured delegate results.
//!
//! Wave 1 keeps the existing `AgentTaskResult` as the active runtime result
//! while introducing frozen naming for future delegate-oriented orchestration.

use serde::{Deserialize, Serialize};

pub use super::result_types::{
    ArtifactRef, ChangedPath, ChangedPathKind, EvidenceItem, OpenIssue, TaskResultStatus,
    TaskStopReason, TaskUsage,
};
use super::{job_types::ResourceLock, result_types::AgentTaskResult, role_profile::SessionSource};

fn default_delegate_session_source() -> SessionSource {
    SessionSource::Delegate
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DelegateInputRef {
    pub kind: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for DelegateInputRef {
    fn default() -> Self {
        Self {
            kind: String::new(),
            value: String::new(),
            description: None,
        }
    }
}

impl DelegateInputRef {
    pub fn normalized(mut self) -> Self {
        self.kind = self.kind.trim().to_string();
        self.value = self.value.trim().to_string();
        self.description = normalize_optional_string(self.description);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ExpectedOutputRef {
    pub kind: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for ExpectedOutputRef {
    fn default() -> Self {
        Self {
            kind: String::new(),
            value: String::new(),
            description: None,
        }
    }
}

impl ExpectedOutputRef {
    pub fn normalized(mut self) -> Self {
        self.kind = self.kind.trim().to_string();
        self.value = self.value.trim().to_string();
        self.description = normalize_optional_string(self.description);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DelegateRequest {
    pub delegate_id: String,
    pub parent_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_turn_id: Option<u32>,
    pub parent_task_id: String,
    pub job_id: String,
    pub goal: String,
    #[serde(default)]
    pub input_refs: Vec<DelegateInputRef>,
    #[serde(default)]
    pub expected_outputs: Vec<ExpectedOutputRef>,
    pub selected_profile_id: String,
    #[serde(default)]
    pub resource_locks: Vec<ResourceLock>,
    #[serde(default = "default_delegate_session_source")]
    pub session_source: SessionSource,
}

impl Default for DelegateRequest {
    fn default() -> Self {
        Self {
            delegate_id: String::new(),
            parent_session_id: String::new(),
            parent_turn_id: None,
            parent_task_id: String::new(),
            job_id: String::new(),
            goal: String::new(),
            input_refs: Vec::new(),
            expected_outputs: Vec::new(),
            selected_profile_id: String::new(),
            resource_locks: Vec::new(),
            session_source: default_delegate_session_source(),
        }
    }
}

impl DelegateRequest {
    pub fn normalized(mut self) -> Self {
        self.delegate_id = self.delegate_id.trim().to_string();
        self.parent_session_id = self.parent_session_id.trim().to_string();
        self.parent_turn_id = self.parent_turn_id.filter(|turn_id| *turn_id > 0);
        self.parent_task_id = self.parent_task_id.trim().to_string();
        self.job_id = self.job_id.trim().to_string();
        self.goal = self.goal.trim().to_string();
        self.input_refs = self
            .input_refs
            .into_iter()
            .map(DelegateInputRef::normalized)
            .collect();
        self.expected_outputs = self
            .expected_outputs
            .into_iter()
            .map(ExpectedOutputRef::normalized)
            .collect();
        self.selected_profile_id = self.selected_profile_id.trim().to_string();
        self.resource_locks = self
            .resource_locks
            .into_iter()
            .map(ResourceLock::normalized)
            .collect();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DelegateResult {
    pub delegate_id: String,
    pub job_id: String,
    pub parent_task_id: String,
    pub goal: String,
    pub status: TaskResultStatus,
    pub stop_reason: TaskStopReason,
    pub result_summary: String,
    #[serde(default)]
    pub changed_paths: Vec<ChangedPath>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactRef>,
    #[serde(default)]
    pub evidence: Vec<EvidenceItem>,
    #[serde(default)]
    pub open_issues: Vec<OpenIssue>,
    #[serde(default)]
    pub next_actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TaskUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
}

impl Default for DelegateResult {
    fn default() -> Self {
        Self {
            delegate_id: String::new(),
            job_id: String::new(),
            parent_task_id: String::new(),
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
            actor_id: None,
        }
    }
}

impl DelegateResult {
    pub fn normalized(mut self) -> Self {
        self.delegate_id = self.delegate_id.trim().to_string();
        self.job_id = self.job_id.trim().to_string();
        self.parent_task_id = self.parent_task_id.trim().to_string();
        self.goal = self.goal.trim().to_string();
        self.result_summary = self.result_summary.trim().to_string();
        self.actor_id = normalize_optional_string(self.actor_id);
        self.next_actions = self
            .next_actions
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        self
    }

    pub fn from_agent_task_result(
        delegate_id: impl Into<String>,
        job_id: impl Into<String>,
        parent_task_id: impl Into<String>,
        result: AgentTaskResult,
    ) -> Self {
        Self {
            delegate_id: delegate_id.into(),
            job_id: job_id.into(),
            parent_task_id: parent_task_id.into(),
            goal: result.goal.clone(),
            status: result.status,
            stop_reason: result.stop_reason,
            result_summary: result.normalized_summary(),
            changed_paths: result.changed_paths,
            artifacts: result.artifacts,
            evidence: result.evidence,
            open_issues: result.open_issues,
            next_actions: result.next_actions,
            usage: result.usage,
            actor_id: normalize_optional_string(Some(result.actor_id)),
        }
        .normalized()
    }

    pub fn into_agent_task_result(self) -> AgentTaskResult {
        let delegate_id = self.delegate_id.clone();
        let parent_task_id = self.parent_task_id.clone();
        let actor_id = self.actor_id.clone().unwrap_or_else(|| delegate_id.clone());
        let task_id = if parent_task_id.is_empty() {
            delegate_id
        } else {
            parent_task_id
        };

        AgentTaskResult {
            task_id,
            actor_id,
            goal: self.goal,
            status: self.status,
            stop_reason: self.stop_reason,
            result_summary: self.result_summary,
            changed_paths: self.changed_paths,
            artifacts: self.artifacts,
            evidence: self.evidence,
            open_issues: self.open_issues,
            next_actions: self.next_actions,
            usage: self.usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegate_request_normalizes_strings_and_locks() {
        let request = DelegateRequest {
            delegate_id: " del_1 ".to_string(),
            parent_session_id: " sess_1 ".to_string(),
            parent_turn_id: Some(7),
            parent_task_id: " task_1 ".to_string(),
            job_id: " job_1 ".to_string(),
            goal: " write summary ".to_string(),
            input_refs: vec![DelegateInputRef {
                kind: " file ".to_string(),
                value: " src/lib.rs ".to_string(),
                description: Some(" main ".to_string()),
            }],
            expected_outputs: vec![ExpectedOutputRef {
                kind: " patch ".to_string(),
                value: " docs ".to_string(),
                description: None,
            }],
            selected_profile_id: " general-worker ".to_string(),
            resource_locks: vec![ResourceLock {
                lock_id: " lock_1 ".to_string(),
                scope: " chapter:1 ".to_string(),
                ..ResourceLock::default()
            }],
            session_source: SessionSource::Delegate,
        }
        .normalized();

        assert_eq!(request.delegate_id, "del_1");
        assert_eq!(request.parent_session_id, "sess_1");
        assert_eq!(request.goal, "write summary");
        assert_eq!(request.input_refs[0].kind, "file");
        assert_eq!(request.expected_outputs[0].value, "docs");
        assert_eq!(request.resource_locks[0].lock_id, "lock_1");
        assert_eq!(request.resource_locks[0].scope, "chapter:1");
    }

    #[test]
    fn delegate_result_bridges_existing_agent_task_result() {
        let result = AgentTaskResult {
            task_id: "feat_1".to_string(),
            actor_id: "wk_1".to_string(),
            goal: "Write chapter".to_string(),
            status: TaskResultStatus::Completed,
            stop_reason: TaskStopReason::Success,
            result_summary: "chapter updated".to_string(),
            next_actions: vec!["run review".to_string()],
            ..AgentTaskResult::default()
        };

        let delegate_result =
            DelegateResult::from_agent_task_result("del_1", "job_1", "feat_1", result.clone());
        let restored = delegate_result.clone().into_agent_task_result();

        assert_eq!(delegate_result.delegate_id, "del_1");
        assert_eq!(delegate_result.job_id, "job_1");
        assert_eq!(delegate_result.parent_task_id, "feat_1");
        assert_eq!(delegate_result.actor_id.as_deref(), Some("wk_1"));
        assert_eq!(restored.task_id, "feat_1");
        assert_eq!(restored.actor_id, "wk_1");
        assert_eq!(restored.result_summary, "chapter updated");
        assert_eq!(restored.next_actions, result.next_actions);
    }
}
