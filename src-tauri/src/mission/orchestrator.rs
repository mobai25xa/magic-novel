//! Mission system - Orchestrator state machine and feature scheduling
//!
//! Core business logic: state transitions, feature dispatch, and task-result recording.
//! Does NOT contain process management (see process_manager.rs).

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::models::AppError;

use super::artifacts;
use super::blockers::WorkflowBlockersDoc;
use super::delegate_types::DelegateResult;
use super::result_types::AgentTaskResult;
use super::types::*;
use super::workflow_types::{
    MissionWorkflowKind, SummaryJobPolicy, WorkflowCreationReason, WorkflowDoc, WorkflowStatus,
};

/// The orchestrator manages mission lifecycle and feature scheduling.
pub struct Orchestrator<'a> {
    project_path: &'a Path,
    mission_id: String,
}

impl<'a> Orchestrator<'a> {
    pub fn new(project_path: &'a Path, mission_id: String) -> Self {
        Self {
            project_path,
            mission_id,
        }
    }

    pub fn mission_id(&self) -> &str {
        &self.mission_id
    }

    // ── Create ──────────────────────────────────────────────────

    /// Create a new mission from user input. Returns the mission_id.
    pub fn create_mission(
        project_path: &Path,
        title: &str,
        mission_text: &str,
        features: Vec<Feature>,
        workflow_kind: MissionWorkflowKind,
        creation_reason: WorkflowCreationReason,
        summary_job_policy: SummaryJobPolicy,
    ) -> Result<String, AppError> {
        let mission_id = format!("mis_{}", uuid::Uuid::new_v4());
        let cwd = project_path.to_string_lossy().to_string();

        let features_doc = FeaturesDoc::new(mission_id.clone(), title.to_string(), features);
        let state_doc = StateDoc::new(mission_id.clone(), cwd);
        let workflow_doc = WorkflowDoc::new(
            mission_id.clone(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
            WorkflowStatus::Draft,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.clone());

        artifacts::init_mission_dir(
            project_path,
            &mission_id,
            mission_text,
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )?;

        tracing::info!(
            target: "mission",
            mission_id = %mission_id,
            title = %title,
            "mission created"
        );

        Ok(mission_id)
    }

    // ── State transitions ───────────────────────────────────────

    /// Transition to a new state. Validates the transition is legal.
    pub fn transition(&self, new_state: MissionState) -> Result<StateDoc, AppError> {
        let mut state = artifacts::read_state(self.project_path, &self.mission_id)?;

        validate_transition(&state.state, &new_state)?;

        let old_state = state.state.clone();
        state.state = new_state.clone();
        state.updated_at = chrono::Utc::now().timestamp_millis();

        artifacts::write_state(self.project_path, &self.mission_id, &state)?;

        tracing::info!(
            target: "mission",
            mission_id = %self.mission_id,
            old_state = ?old_state,
            new_state = ?new_state,
            "mission state transition"
        );

        Ok(state)
    }

    // ── Feature scheduling ──────────────────────────────────────

    /// Find the next pending feature. Returns None if all done/cancelled/failed.
    pub fn next_pending_feature(&self) -> Result<Option<Feature>, AppError> {
        let features_doc = artifacts::read_features(self.project_path, &self.mission_id)?;
        let state = artifacts::read_state(self.project_path, &self.mission_id)?;
        let running_feature_ids = state
            .assignments
            .values()
            .map(|a| a.feature_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let ready = select_ready_feature_indices(&features_doc.features, &running_feature_ids)?;
        let next = ready
            .into_iter()
            .next()
            .map(|idx| features_doc.features[idx].clone());
        Ok(next)
    }

    pub fn ready_pending_features(&self, limit: usize) -> Result<Vec<Feature>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let features_doc = artifacts::read_features(self.project_path, &self.mission_id)?;
        let state = artifacts::read_state(self.project_path, &self.mission_id)?;
        let running_feature_ids = state
            .assignments
            .values()
            .map(|a| a.feature_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let ready = select_ready_feature_indices(&features_doc.features, &running_feature_ids)?;

        Ok(ready
            .into_iter()
            .take(limit)
            .map(|idx| features_doc.features[idx].clone())
            .collect())
    }

    /// Update a feature's status.
    pub fn update_feature_status(
        &self,
        feature_id: &str,
        new_status: FeatureStatus,
    ) -> Result<(), AppError> {
        let mut doc = artifacts::read_features(self.project_path, &self.mission_id)?;
        let feature = doc
            .features
            .iter_mut()
            .find(|f| f.id == feature_id)
            .ok_or_else(|| AppError::not_found(format!("feature not found: {feature_id}")))?;
        feature.status = new_status;
        artifacts::write_features(self.project_path, &self.mission_id, &doc)
    }

    /// Mark the feature in state.json and set state to Running.
    pub fn start_feature(
        &self,
        feature_id: &str,
        worker_id: &str,
        attempt: u32,
    ) -> Result<StateDoc, AppError> {
        self.update_feature_status(feature_id, FeatureStatus::InProgress)?;

        let mut state = artifacts::read_state(self.project_path, &self.mission_id)?;
        let now = chrono::Utc::now().timestamp_millis();
        state.current_feature_id = Some(feature_id.to_string());
        state.current_worker_id = Some(worker_id.to_string());
        state.assignments.insert(
            worker_id.to_string(),
            WorkerAssignment {
                feature_id: feature_id.to_string(),
                attempt,
                started_at: now,
                last_heartbeat_at: now,
            },
        );
        state.state = MissionState::Running;
        state.updated_at = now;

        artifacts::write_state(self.project_path, &self.mission_id, &state)?;

        tracing::info!(
            target: "mission",
            mission_id = %self.mission_id,
            feature_id = %feature_id,
            worker_id = %worker_id,
            attempt = attempt,
            "feature started"
        );

        Ok(state)
    }

    /// Record a structured task result, persist a derived handoff, and advance mission state.
    pub fn complete_feature_result(
        &self,
        feature_id: &str,
        worker_id: &str,
        result: &AgentTaskResult,
    ) -> Result<MissionState, AppError> {
        let handoff = result.to_handoff_entry(feature_id, worker_id);
        artifacts::append_task_result(self.project_path, &self.mission_id, result)?;
        artifacts::append_handoff(self.project_path, &self.mission_id, &handoff)?;

        let new_feature_status = match result.status {
            super::result_types::TaskResultStatus::Completed => FeatureStatus::Completed,
            super::result_types::TaskResultStatus::Failed => FeatureStatus::Failed,
            super::result_types::TaskResultStatus::Cancelled
            | super::result_types::TaskResultStatus::Blocked => FeatureStatus::Pending,
        };
        self.update_feature_status(feature_id, new_feature_status)?;

        let mut state = artifacts::read_state(self.project_path, &self.mission_id)?;
        state.assignments.remove(worker_id);

        if state.current_worker_id.as_deref() == Some(worker_id) {
            if let Some((worker_id, assignment)) = state
                .assignments
                .iter()
                .max_by_key(|(_, assignment)| assignment.started_at)
            {
                state.current_worker_id = Some(worker_id.clone());
                state.current_feature_id = Some(assignment.feature_id.clone());
            } else {
                state.current_worker_id = None;
                state.current_feature_id = None;
            }
        }

        state.updated_at = chrono::Utc::now().timestamp_millis();

        let features_doc = artifacts::read_features(self.project_path, &self.mission_id)?;
        let has_pending = features_doc
            .features
            .iter()
            .any(|f| f.status == FeatureStatus::Pending);
        let has_running = !state.assignments.is_empty();

        let has_failed = features_doc
            .features
            .iter()
            .any(|f| f.status == FeatureStatus::Failed);

        state.state = if matches!(
            result.status,
            super::result_types::TaskResultStatus::Cancelled
                | super::result_types::TaskResultStatus::Blocked
        ) {
            MissionState::Paused
        } else if has_pending {
            if has_running {
                MissionState::Running
            } else {
                MissionState::OrchestratorTurn
            }
        } else if has_running {
            MissionState::Running
        } else if has_failed {
            MissionState::Failed
        } else {
            MissionState::Completed
        };

        artifacts::write_state(self.project_path, &self.mission_id, &state)?;

        tracing::info!(
            target: "mission",
            mission_id = %self.mission_id,
            feature_id = %feature_id,
            worker_id = %worker_id,
            ok = result.is_ok(),
            next_state = ?state.state,
            "feature completed"
        );

        Ok(state.state)
    }

    /// Record a structured delegate result while keeping the legacy task-result
    /// artifact pipeline intact for compatibility with existing mission storage.
    pub fn complete_feature_delegate_result(
        &self,
        feature_id: &str,
        worker_id: &str,
        result: &DelegateResult,
    ) -> Result<MissionState, AppError> {
        let task_result = result.clone().into_agent_task_result();
        self.complete_feature_result(feature_id, worker_id, &task_result)
    }

    /// Check if all features are completed or failed/cancelled and no worker assignment remains.
    pub fn is_finished(&self) -> Result<bool, AppError> {
        let doc = artifacts::read_features(self.project_path, &self.mission_id)?;
        let state = artifacts::read_state(self.project_path, &self.mission_id)?;
        Ok(doc.features.iter().all(|f| {
            matches!(
                f.status,
                FeatureStatus::Completed | FeatureStatus::Failed | FeatureStatus::Cancelled
            )
        }) && state.assignments.is_empty())
    }

    /// Get current state snapshot.
    pub fn get_state(&self) -> Result<StateDoc, AppError> {
        artifacts::read_state(self.project_path, &self.mission_id)
    }

    /// Get features doc snapshot.
    pub fn get_features(&self) -> Result<FeaturesDoc, AppError> {
        artifacts::read_features(self.project_path, &self.mission_id)
    }
}

// ── State transition validation ─────────────────────────────────

/// Legal state transitions:
///   awaiting_input    → initializing
///   initializing      → running | paused | blocked
///   running           → paused | orchestrator_turn | completed | blocked
///   paused            → running | completed | blocked
///   orchestrator_turn → running | completed | paused | blocked
fn validate_transition(from: &MissionState, to: &MissionState) -> Result<(), AppError> {
    let valid = matches!(
        (from, to),
        (MissionState::AwaitingInput, MissionState::Initializing)
            | (MissionState::Initializing, MissionState::Running)
            | (MissionState::Initializing, MissionState::Paused)
            | (MissionState::Initializing, MissionState::Blocked)
            | (MissionState::Initializing, MissionState::WaitingUser)
            | (MissionState::Initializing, MissionState::WaitingReview)
            | (
                MissionState::Initializing,
                MissionState::WaitingKnowledgeDecision
            )
            | (MissionState::Initializing, MissionState::Failed)
            | (MissionState::Initializing, MissionState::Cancelled)
            | (MissionState::Running, MissionState::Paused)
            | (MissionState::Running, MissionState::OrchestratorTurn)
            | (MissionState::Running, MissionState::Completed)
            | (MissionState::Running, MissionState::Blocked)
            | (MissionState::Running, MissionState::WaitingUser)
            | (MissionState::Running, MissionState::WaitingReview)
            | (
                MissionState::Running,
                MissionState::WaitingKnowledgeDecision
            )
            | (MissionState::Running, MissionState::Failed)
            | (MissionState::Running, MissionState::Cancelled)
            | (MissionState::Paused, MissionState::Running)
            | (MissionState::Paused, MissionState::Completed)
            | (MissionState::Paused, MissionState::Blocked)
            | (MissionState::Paused, MissionState::WaitingUser)
            | (MissionState::Paused, MissionState::WaitingReview)
            | (MissionState::Paused, MissionState::WaitingKnowledgeDecision)
            | (MissionState::Paused, MissionState::Failed)
            | (MissionState::Paused, MissionState::Cancelled)
            | (MissionState::OrchestratorTurn, MissionState::Running)
            | (MissionState::OrchestratorTurn, MissionState::Completed)
            | (MissionState::OrchestratorTurn, MissionState::Paused)
            | (MissionState::OrchestratorTurn, MissionState::Blocked)
            | (MissionState::OrchestratorTurn, MissionState::WaitingUser)
            | (MissionState::OrchestratorTurn, MissionState::WaitingReview)
            | (
                MissionState::OrchestratorTurn,
                MissionState::WaitingKnowledgeDecision
            )
            | (MissionState::OrchestratorTurn, MissionState::Failed)
            | (MissionState::OrchestratorTurn, MissionState::Cancelled)
            | (MissionState::Blocked, MissionState::Paused)
            | (MissionState::Blocked, MissionState::Running)
            | (MissionState::Blocked, MissionState::Completed)
            | (MissionState::Blocked, MissionState::Failed)
            | (MissionState::Blocked, MissionState::Cancelled)
            | (MissionState::WaitingUser, MissionState::Paused)
            | (MissionState::WaitingUser, MissionState::Running)
            | (MissionState::WaitingUser, MissionState::Completed)
            | (MissionState::WaitingUser, MissionState::Failed)
            | (MissionState::WaitingUser, MissionState::Cancelled)
            | (MissionState::WaitingReview, MissionState::Paused)
            | (MissionState::WaitingReview, MissionState::Running)
            | (MissionState::WaitingReview, MissionState::Completed)
            | (MissionState::WaitingReview, MissionState::Failed)
            | (MissionState::WaitingReview, MissionState::Cancelled)
            | (MissionState::WaitingKnowledgeDecision, MissionState::Paused)
            | (
                MissionState::WaitingKnowledgeDecision,
                MissionState::Running
            )
            | (
                MissionState::WaitingKnowledgeDecision,
                MissionState::Completed
            )
            | (MissionState::WaitingKnowledgeDecision, MissionState::Failed)
            | (
                MissionState::WaitingKnowledgeDecision,
                MissionState::Cancelled
            )
    );

    if !valid {
        return Err(AppError::invalid_argument(format!(
            "invalid mission state transition: {:?} -> {:?}",
            from, to
        )));
    }
    Ok(())
}

fn normalize_write_paths(write_paths: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    for raw in write_paths {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut p = trimmed.replace('\\', "/");
        while p.starts_with("./") {
            p = p[2..].to_string();
        }
        while p.contains("//") {
            p = p.replace("//", "/");
        }
        if p.starts_with('/') {
            continue;
        }
        if p.split('/').any(|seg| seg == "..") {
            continue;
        }

        if seen.insert(p.clone()) {
            normalized.push(p);
        }
    }

    normalized
}

fn writes_conflict(left_paths: &[String], right_paths: &[String]) -> bool {
    let left = normalize_write_paths(left_paths);
    let right = normalize_write_paths(right_paths);

    if left.is_empty() || right.is_empty() {
        return true;
    }

    let right_set = right.into_iter().collect::<HashSet<_>>();
    left.into_iter().any(|p| right_set.contains(&p))
}

fn select_ready_feature_indices(
    features: &[Feature],
    running_feature_ids: &HashSet<&str>,
) -> Result<Vec<usize>, AppError> {
    let order = schedule_feature_indices(features)?;

    let mut status_map = HashMap::new();
    for feature in features {
        let status = if running_feature_ids.contains(feature.id.as_str()) {
            FeatureStatus::InProgress
        } else {
            feature.status.clone()
        };
        status_map.insert(feature.id.as_str(), status);
    }

    let running_features = features
        .iter()
        .filter(|f| running_feature_ids.contains(f.id.as_str()))
        .collect::<Vec<_>>();

    let mut selected = Vec::new();
    let mut selected_features = Vec::new();

    for idx in order {
        let feature = &features[idx];
        if feature.status != FeatureStatus::Pending {
            continue;
        }

        let deps_ready = feature
            .depends_on
            .iter()
            .all(|dep| matches!(status_map.get(dep.as_str()), Some(FeatureStatus::Completed)));
        if !deps_ready {
            continue;
        }

        let conflict_running = running_features
            .iter()
            .any(|running| writes_conflict(&feature.write_paths, &running.write_paths));
        if conflict_running {
            continue;
        }

        let conflict_selected = selected_features
            .iter()
            .any(|already: &&Feature| writes_conflict(&feature.write_paths, &already.write_paths));
        if conflict_selected {
            continue;
        }

        selected.push(idx);
        selected_features.push(feature);
    }

    Ok(selected)
}

fn schedule_feature_indices(features: &[Feature]) -> Result<Vec<usize>, AppError> {
    let mut id_to_idx: HashMap<&str, usize> = HashMap::new();
    for (idx, feature) in features.iter().enumerate() {
        if id_to_idx.insert(feature.id.as_str(), idx).is_some() {
            return Err(AppError::invalid_argument(format!(
                "duplicate feature id: {}",
                feature.id
            )));
        }
    }

    let mut indegree = vec![0usize; features.len()];
    let mut graph: Vec<Vec<usize>> = vec![Vec::new(); features.len()];

    for (idx, feature) in features.iter().enumerate() {
        for dep in &feature.depends_on {
            let dep_idx = id_to_idx.get(dep.as_str()).copied().ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "feature '{}' depends on unknown feature '{}'",
                    feature.id, dep
                ))
            })?;
            graph[dep_idx].push(idx);
            indegree[idx] += 1;
        }
    }

    let mut queue: VecDeque<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(idx, deg)| if *deg == 0 { Some(idx) } else { None })
        .collect();

    let mut order = Vec::with_capacity(features.len());
    while let Some(node) = queue.pop_front() {
        order.push(node);
        for &next in &graph[node] {
            indegree[next] -= 1;
            if indegree[next] == 0 {
                queue.push_back(next);
            }
        }
    }

    if order.len() != features.len() {
        return Err(AppError::invalid_argument(
            "feature dependency cycle detected".to_string(),
        ));
    }

    Ok(order)
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::delegate_types::DelegateResult;
    use crate::mission::result_types::{
        AgentTaskResult, OpenIssue, TaskResultStatus, TaskStopReason,
    };
    use std::fs;
    use std::path::PathBuf;

    fn default_workflow_args() -> (
        MissionWorkflowKind,
        WorkflowCreationReason,
        SummaryJobPolicy,
    ) {
        (
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
        )
    }

    fn temp_project_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("magic_orch_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_features() -> Vec<Feature> {
        vec![
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::Pending,
                description: "Write chapter 1".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: vec!["Chapter 1 exists".to_string()],
                verification_steps: Vec::new(),
                write_paths: vec!["chapters/ch1.md".to_string()],
            },
            Feature {
                id: "f2".to_string(),
                status: FeatureStatus::Pending,
                description: "Write chapter 2".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: vec!["f1".to_string()],
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["chapters/ch2.md".to_string()],
            },
        ]
    }

    fn build_result(
        feature_id: &str,
        worker_id: &str,
        status: TaskResultStatus,
        summary: &str,
        issues: Vec<&str>,
    ) -> AgentTaskResult {
        AgentTaskResult {
            task_id: feature_id.to_string(),
            actor_id: worker_id.to_string(),
            goal: format!("feature {feature_id}"),
            status,
            stop_reason: match status {
                TaskResultStatus::Completed => TaskStopReason::Success,
                TaskResultStatus::Failed => TaskStopReason::Error,
                TaskResultStatus::Cancelled => TaskStopReason::Cancelled,
                TaskResultStatus::Blocked => TaskStopReason::Blocked,
            },
            result_summary: summary.to_string(),
            open_issues: issues
                .into_iter()
                .map(|issue| OpenIssue {
                    code: None,
                    summary: issue.to_string(),
                    blocking: true,
                })
                .collect(),
            ..AgentTaskResult::default()
        }
    }

    #[test]
    fn test_create_mission() {
        let project = temp_project_dir();
        let (workflow_kind, creation_reason, summary_job_policy) = default_workflow_args();
        let mission_id = Orchestrator::create_mission(
            &project,
            "Test",
            "# Mission",
            sample_features(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
        )
        .unwrap();
        assert!(mission_id.starts_with("mis_"));

        // Verify files
        assert!(artifacts::features_path(&project, &mission_id).exists());
        assert!(artifacts::state_path(&project, &mission_id).exists());

        let state = artifacts::read_state(&project, &mission_id).unwrap();
        assert_eq!(state.state, MissionState::AwaitingInput);
        let workflow = artifacts::read_workflow(&project, &mission_id).unwrap();
        assert_eq!(workflow.status, WorkflowStatus::Draft);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_valid_transitions() {
        assert!(
            validate_transition(&MissionState::AwaitingInput, &MissionState::Initializing).is_ok()
        );
        assert!(validate_transition(&MissionState::Initializing, &MissionState::Running).is_ok());
        assert!(validate_transition(&MissionState::Running, &MissionState::Paused).is_ok());
        assert!(
            validate_transition(&MissionState::Running, &MissionState::OrchestratorTurn).is_ok()
        );
        assert!(validate_transition(&MissionState::Running, &MissionState::Completed).is_ok());
        assert!(validate_transition(&MissionState::Paused, &MissionState::Running).is_ok());
        assert!(validate_transition(&MissionState::Paused, &MissionState::Completed).is_ok());
        assert!(
            validate_transition(&MissionState::OrchestratorTurn, &MissionState::Running).is_ok()
        );
        assert!(
            validate_transition(&MissionState::OrchestratorTurn, &MissionState::Completed).is_ok()
        );
        assert!(
            validate_transition(&MissionState::OrchestratorTurn, &MissionState::Paused).is_ok()
        );
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(validate_transition(&MissionState::AwaitingInput, &MissionState::Running).is_err());
        assert!(validate_transition(&MissionState::Completed, &MissionState::Running).is_err());
        assert!(validate_transition(&MissionState::Running, &MissionState::AwaitingInput).is_err());
        assert!(validate_transition(&MissionState::Paused, &MissionState::Initializing).is_err());
    }

    #[test]
    fn test_full_lifecycle() {
        let project = temp_project_dir();
        let (workflow_kind, creation_reason, summary_job_policy) = default_workflow_args();
        let mission_id = Orchestrator::create_mission(
            &project,
            "Life",
            "# Lifecycle",
            sample_features(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
        )
        .unwrap();
        let orch = Orchestrator::new(&project, mission_id.clone());

        // awaiting_input → initializing
        orch.transition(MissionState::Initializing).unwrap();
        assert_eq!(orch.get_state().unwrap().state, MissionState::Initializing);

        // Find next pending feature
        let f = orch.next_pending_feature().unwrap().unwrap();
        assert_eq!(f.id, "f1");

        // Start feature
        let state = orch.start_feature("f1", "wk_001", 0).unwrap();
        assert_eq!(state.state, MissionState::Running);
        assert_eq!(state.current_feature_id, Some("f1".to_string()));
        assert_eq!(state.current_worker_id, Some("wk_001".to_string()));

        // Complete feature successfully → should go to OrchestratorTurn (f2 pending)
        let next_state = orch
            .complete_feature_result(
                "f1",
                "wk_001",
                &build_result(
                    "f1",
                    "wk_001",
                    TaskResultStatus::Completed,
                    "done",
                    Vec::new(),
                ),
            )
            .unwrap();
        assert_eq!(next_state, MissionState::OrchestratorTurn);
        assert!(!orch.is_finished().unwrap());

        // Start and complete f2
        orch.transition(MissionState::Running).unwrap();
        orch.start_feature("f2", "wk_002", 0).unwrap();
        let final_state = orch
            .complete_feature_result(
                "f2",
                "wk_002",
                &build_result(
                    "f2",
                    "wk_002",
                    TaskResultStatus::Completed,
                    "done",
                    Vec::new(),
                ),
            )
            .unwrap();
        assert_eq!(final_state, MissionState::Completed);
        assert!(orch.is_finished().unwrap());

        // Verify handoffs
        let task_results = artifacts::read_task_results(&project, &mission_id).unwrap();
        assert_eq!(task_results.len(), 2);

        let handoffs = artifacts::read_handoffs(&project, &mission_id).unwrap();
        assert_eq!(handoffs.len(), 2);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_feature_not_found() {
        let project = temp_project_dir();
        let (workflow_kind, creation_reason, summary_job_policy) = default_workflow_args();
        let mission_id = Orchestrator::create_mission(
            &project,
            "T",
            "t",
            sample_features(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
        )
        .unwrap();
        let orch = Orchestrator::new(&project, mission_id);

        let result = orch.update_feature_status("f_nonexistent", FeatureStatus::Completed);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_failed_feature_advances() {
        let project = temp_project_dir();
        let (workflow_kind, creation_reason, summary_job_policy) = default_workflow_args();
        let mission_id = Orchestrator::create_mission(
            &project,
            "T",
            "t",
            sample_features(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
        )
        .unwrap();
        let orch = Orchestrator::new(&project, mission_id.clone());

        orch.transition(MissionState::Initializing).unwrap();
        orch.start_feature("f1", "wk_1", 0).unwrap();

        let next = orch
            .complete_feature_result(
                "f1",
                "wk_1",
                &build_result(
                    "f1",
                    "wk_1",
                    TaskResultStatus::Failed,
                    "crashed",
                    vec!["timeout"],
                ),
            )
            .unwrap();
        assert_eq!(next, MissionState::OrchestratorTurn);

        // f1 should be Failed
        let features = orch.get_features().unwrap();
        assert_eq!(features.features[0].status, FeatureStatus::Failed);

        // f2 is blocked in this state until orchestrator assigns a worker
        let pending = orch.next_pending_feature().unwrap();
        assert!(pending.is_none());

        let ready = orch.ready_pending_features(5).unwrap();
        assert_eq!(ready.len(), 0);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_blocked_feature_pauses_and_keeps_feature_pending() {
        let project = temp_project_dir();
        let (workflow_kind, creation_reason, summary_job_policy) = default_workflow_args();
        let mission_id = Orchestrator::create_mission(
            &project,
            "Blocked",
            "blocked flow",
            sample_features(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
        )
        .unwrap();
        let orch = Orchestrator::new(&project, mission_id.clone());

        orch.transition(MissionState::Initializing).unwrap();
        orch.start_feature("f1", "wk_1", 0).unwrap();

        let next = orch
            .complete_feature_result(
                "f1",
                "wk_1",
                &build_result(
                    "f1",
                    "wk_1",
                    TaskResultStatus::Blocked,
                    "waiting for user clarification",
                    vec!["clarification required"],
                ),
            )
            .unwrap();
        assert_eq!(next, MissionState::Paused);

        let state = orch.get_state().unwrap();
        assert_eq!(state.state, MissionState::Paused);
        assert!(state.assignments.is_empty());
        assert!(state.current_worker_id.is_none());

        let features = orch.get_features().unwrap();
        assert_eq!(features.features[0].status, FeatureStatus::Pending);

        let task_results = artifacts::read_task_results(&project, &mission_id).unwrap();
        assert_eq!(task_results.len(), 1);
        assert_eq!(task_results[0].status, TaskResultStatus::Blocked);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_cancelled_feature_pauses_and_keeps_feature_pending() {
        let project = temp_project_dir();
        let (workflow_kind, creation_reason, summary_job_policy) = default_workflow_args();
        let mission_id = Orchestrator::create_mission(
            &project,
            "Cancelled",
            "cancelled flow",
            sample_features(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
        )
        .unwrap();
        let orch = Orchestrator::new(&project, mission_id.clone());

        orch.transition(MissionState::Initializing).unwrap();
        orch.start_feature("f1", "wk_1", 0).unwrap();

        let next = orch
            .complete_feature_result(
                "f1",
                "wk_1",
                &build_result(
                    "f1",
                    "wk_1",
                    TaskResultStatus::Cancelled,
                    "delegate cancelled",
                    vec!["delegate cancelled"],
                ),
            )
            .unwrap();
        assert_eq!(next, MissionState::Paused);

        let state = orch.get_state().unwrap();
        assert_eq!(state.state, MissionState::Paused);

        let features = orch.get_features().unwrap();
        assert_eq!(features.features[0].status, FeatureStatus::Pending);

        let task_results = artifacts::read_task_results(&project, &mission_id).unwrap();
        assert_eq!(task_results.len(), 1);
        assert_eq!(task_results[0].status, TaskResultStatus::Cancelled);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_delegate_result_completion_uses_structured_result() {
        let project = temp_project_dir();
        let (workflow_kind, creation_reason, summary_job_policy) = default_workflow_args();
        let mission_id = Orchestrator::create_mission(
            &project,
            "Delegate",
            "delegate flow",
            sample_features(),
            workflow_kind,
            creation_reason,
            summary_job_policy,
        )
        .unwrap();
        let orch = Orchestrator::new(&project, mission_id.clone());

        orch.transition(MissionState::Initializing).unwrap();
        orch.start_feature("f1", "wk_delegate", 0).unwrap();

        let delegate_result = DelegateResult::from_agent_task_result(
            "del_1",
            mission_id.clone(),
            "f1",
            build_result(
                "f1",
                "wk_delegate",
                TaskResultStatus::Completed,
                "delegate done",
                Vec::new(),
            ),
        );

        let next_state = orch
            .complete_feature_delegate_result("f1", "wk_delegate", &delegate_result)
            .unwrap();

        assert_eq!(next_state, MissionState::OrchestratorTurn);

        let task_results = artifacts::read_task_results(&project, &mission_id).unwrap();
        assert_eq!(task_results.len(), 1);
        assert_eq!(task_results[0].task_id, "f1");
        assert_eq!(task_results[0].result_summary, "delegate done");

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_schedule_feature_indices_orders_by_dependency() {
        let features = vec![
            Feature {
                id: "f2".to_string(),
                status: FeatureStatus::Pending,
                description: "Second".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: vec!["f1".to_string()],
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["chapters/ch2.md".to_string()],
            },
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::Pending,
                description: "First".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["chapters/ch1.md".to_string()],
            },
        ];

        let order = schedule_feature_indices(&features).unwrap();
        let ordered_ids = order
            .into_iter()
            .map(|idx| features[idx].id.clone())
            .collect::<Vec<_>>();
        assert_eq!(ordered_ids, vec!["f1".to_string(), "f2".to_string()]);
    }

    #[test]
    fn test_schedule_feature_indices_detects_cycle() {
        let features = vec![
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::Pending,
                description: "First".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: vec!["f2".to_string()],
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: Vec::new(),
            },
            Feature {
                id: "f2".to_string(),
                status: FeatureStatus::Pending,
                description: "Second".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: vec!["f1".to_string()],
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: Vec::new(),
            },
        ];

        let err = schedule_feature_indices(&features).unwrap_err();
        assert!(err.message.contains("dependency cycle"));
    }

    #[test]
    fn test_schedule_feature_indices_unknown_dependency_error() {
        let features = vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "Only".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: vec!["missing".to_string()],
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }];

        let err = schedule_feature_indices(&features).unwrap_err();
        assert!(err.message.contains("depends on unknown feature"));
    }

    #[test]
    fn test_select_ready_features_filters_conflicts() {
        let features = vec![
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::Pending,
                description: "A".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["src/a.ts".to_string()],
            },
            Feature {
                id: "f2".to_string(),
                status: FeatureStatus::Pending,
                description: "B".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["src/a.ts".to_string()],
            },
            Feature {
                id: "f3".to_string(),
                status: FeatureStatus::Pending,
                description: "C".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["src/c.ts".to_string()],
            },
        ];

        let running = HashSet::new();
        let ready = select_ready_feature_indices(&features, &running).unwrap();
        let ids = ready
            .into_iter()
            .map(|idx| features[idx].id.clone())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"f1".to_string()));
        assert!(ids.contains(&"f3".to_string()));
        assert!(!ids.contains(&"f2".to_string()));
    }

    #[test]
    fn test_select_ready_features_empty_write_paths_blocks_parallel() {
        let features = vec![
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::Pending,
                description: "A".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: Vec::new(),
            },
            Feature {
                id: "f2".to_string(),
                status: FeatureStatus::Pending,
                description: "B".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["src/b.ts".to_string()],
            },
        ];

        let running = HashSet::new();
        let ready = select_ready_feature_indices(&features, &running).unwrap();
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn test_select_ready_features_respects_running_conflicts() {
        let features = vec![
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::InProgress,
                description: "A".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["src/a.ts".to_string()],
            },
            Feature {
                id: "f2".to_string(),
                status: FeatureStatus::Pending,
                description: "B".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["src/a.ts".to_string()],
            },
            Feature {
                id: "f3".to_string(),
                status: FeatureStatus::Pending,
                description: "C".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["src/c.ts".to_string()],
            },
        ];

        let running = HashSet::from(["f1"]);
        let ready = select_ready_feature_indices(&features, &running).unwrap();
        let ids = ready
            .into_iter()
            .map(|idx| features[idx].id.clone())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["f3".to_string()]);
    }
}
