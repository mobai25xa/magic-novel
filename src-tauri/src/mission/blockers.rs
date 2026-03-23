use serde::{Deserialize, Serialize};

use crate::knowledge::types::PendingKnowledgeDecision;
use crate::review::types::ReviewDecisionRequest;

pub const WORKFLOW_BLOCKERS_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBlockerKind {
    ReviewGate,
    KnowledgeDecision,
    UserClarification,
    ExternalDependency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowBlocker {
    pub blocker_id: String,
    pub kind: WorkflowBlockerKind,
    pub summary: String,
    pub blocking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_task_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl WorkflowBlocker {
    pub fn review_gate(request: &ReviewDecisionRequest) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            blocker_id: format!("review:{}", request.review_id),
            kind: WorkflowBlockerKind::ReviewGate,
            summary: format!("Pending review decision for {}", request.scope_ref),
            blocking: true,
            feature_id: request.feature_id.clone(),
            related_task_ids: request.feature_id.iter().cloned().collect(),
            wave_id: request
                .feature_id
                .as_ref()
                .map(|feature_id| format!("feature:{feature_id}")),
            created_at: request.created_at,
            updated_at: now,
        }
    }

    pub fn knowledge_decision(request: &PendingKnowledgeDecision) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            blocker_id: format!("knowledge:{}", request.delta_id),
            kind: WorkflowBlockerKind::KnowledgeDecision,
            summary: format!("Pending knowledge decision for {}", request.scope_ref),
            blocking: true,
            feature_id: None,
            related_task_ids: Vec::new(),
            wave_id: None,
            created_at: request.created_at,
            updated_at: now,
        }
    }

    pub fn user_clarification(
        mission_id: &str,
        summary: impl Into<String>,
        feature_id: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            blocker_id: format!("clarification:{mission_id}"),
            kind: WorkflowBlockerKind::UserClarification,
            summary: summary.into(),
            blocking: true,
            feature_id,
            related_task_ids: Vec::new(),
            wave_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn external_dependency(
        mission_id: &str,
        summary: impl Into<String>,
        feature_id: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            blocker_id: format!("external:{mission_id}"),
            kind: WorkflowBlockerKind::ExternalDependency,
            summary: summary.into(),
            blocking: true,
            feature_id,
            related_task_ids: Vec::new(),
            wave_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_related_task_ids(
        mut self,
        related_task_ids: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut deduped = std::collections::BTreeSet::new();
        for task_id in related_task_ids {
            let trimmed = task_id.trim();
            if !trimmed.is_empty() {
                deduped.insert(trimmed.to_string());
            }
        }
        self.related_task_ids = deduped.into_iter().collect();
        self
    }

    pub fn with_wave_id(mut self, wave_id: Option<String>) -> Self {
        self.wave_id = wave_id
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty());
        self
    }

    pub fn with_timestamps(mut self, created_at: i64, updated_at: i64) -> Self {
        self.created_at = created_at;
        self.updated_at = updated_at;
        self
    }

    pub fn blocks_task(&self, task_id: &str) -> bool {
        if !self.blocking {
            return false;
        }

        let task_id = task_id.trim();
        if task_id.is_empty() {
            return self.feature_id.is_none() && self.related_task_ids.is_empty();
        }

        if self
            .feature_id
            .as_deref()
            .is_some_and(|feature_id| feature_id.trim().eq_ignore_ascii_case(task_id))
        {
            return true;
        }

        if self
            .related_task_ids
            .iter()
            .map(|candidate| candidate.trim())
            .any(|candidate| candidate.eq_ignore_ascii_case(task_id))
        {
            return true;
        }

        self.feature_id.is_none() && self.related_task_ids.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBlockersDoc {
    pub schema_version: i32,
    pub mission_id: String,
    #[serde(default)]
    pub blockers: Vec<WorkflowBlocker>,
    pub updated_at: i64,
}

impl WorkflowBlockersDoc {
    pub fn empty(mission_id: String) -> Self {
        Self {
            schema_version: WORKFLOW_BLOCKERS_SCHEMA_VERSION,
            mission_id,
            blockers: Vec::new(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

pub fn derive_blockers(
    mission_id: &str,
    pending_review: Option<&ReviewDecisionRequest>,
    pending_knowledge: Option<&PendingKnowledgeDecision>,
) -> WorkflowBlockersDoc {
    let mut blockers = Vec::new();
    if let Some(request) = pending_review {
        blockers.push(WorkflowBlocker::review_gate(request));
    }
    if let Some(request) = pending_knowledge {
        blockers.push(WorkflowBlocker::knowledge_decision(request));
    }

    WorkflowBlockersDoc {
        schema_version: WORKFLOW_BLOCKERS_SCHEMA_VERSION,
        mission_id: mission_id.to_string(),
        blockers,
        updated_at: chrono::Utc::now().timestamp_millis(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::types::{PendingKnowledgeDecision, KNOWLEDGE_SCHEMA_VERSION};
    use crate::review::types::ReviewDecisionRequest;

    #[test]
    fn derive_blockers_collects_pending_decisions() {
        let review = ReviewDecisionRequest {
            schema_version: 1,
            review_id: "rev_1".to_string(),
            feature_id: Some("feat_1".to_string()),
            scope_ref: "chapter:1".to_string(),
            target_refs: None,
            question: "fix?".to_string(),
            options: vec!["auto_fix".to_string()],
            context_summary: Vec::new(),
            created_at: 10,
        };
        let knowledge = PendingKnowledgeDecision {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            bundle_id: "bundle_1".to_string(),
            delta_id: "delta_1".to_string(),
            scope_ref: "chapter:1".to_string(),
            conflicts: Vec::new(),
            created_at: 20,
        };

        let doc = derive_blockers("mis_1", Some(&review), Some(&knowledge));

        assert_eq!(doc.blockers.len(), 2);
        assert_eq!(doc.blockers[0].kind, WorkflowBlockerKind::ReviewGate);
        assert_eq!(doc.blockers[1].kind, WorkflowBlockerKind::KnowledgeDecision);
    }

    #[test]
    fn transient_blocker_constructors_cover_remaining_kinds() {
        let clarification = WorkflowBlocker::user_clarification(
            "mis_1",
            "waiting for user answer",
            Some("feat_1".to_string()),
        );
        let external =
            WorkflowBlocker::external_dependency("mis_1", "waiting for external dependency", None);

        assert_eq!(clarification.kind, WorkflowBlockerKind::UserClarification);
        assert_eq!(clarification.feature_id.as_deref(), Some("feat_1"));
        assert_eq!(external.kind, WorkflowBlockerKind::ExternalDependency);
        assert!(external.feature_id.is_none());
    }

    #[test]
    fn scoped_blocker_targets_related_tasks() {
        let blocker = WorkflowBlocker::external_dependency(
            "mis_1",
            "blocked by external review",
            Some("feat_1".to_string()),
        )
        .with_related_task_ids(vec!["feat_2".to_string()]);

        assert!(blocker.blocks_task("feat_1"));
        assert!(blocker.blocks_task("feat_2"));
        assert!(!blocker.blocks_task("feat_3"));
    }

    #[test]
    fn blocks_task_matches_trimmed_related_ids_case_insensitively() {
        let mut blocker = WorkflowBlocker::external_dependency(
            "mis_1",
            "blocked by external review",
            Some("feat_1".to_string()),
        );
        blocker.related_task_ids = vec!["  FeAt_2  ".to_string()];

        assert!(blocker.blocks_task("feat_2"));
        assert!(blocker.blocks_task("FEAT_2"));
    }
}
