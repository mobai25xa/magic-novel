//! Mission system - Orchestrator state machine and feature scheduling
//!
//! Core business logic: state transitions, feature dispatch, handoff recording.
//! Does NOT contain process management (see process_manager.rs).

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::models::AppError;

use super::artifacts;
use super::types::*;

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
    ) -> Result<String, AppError> {
        let mission_id = format!("mis_{}", uuid::Uuid::new_v4());
        let cwd = project_path.to_string_lossy().to_string();

        let features_doc = FeaturesDoc::new(mission_id.clone(), title.to_string(), features);
        let state_doc = StateDoc::new(mission_id.clone(), cwd);

        artifacts::init_mission_dir(
            project_path,
            &mission_id,
            mission_text,
            &features_doc,
            &state_doc,
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

    /// Record a handoff and advance mission state.
    pub fn complete_feature(&self, handoff: &HandoffEntry) -> Result<MissionState, AppError> {
        artifacts::append_handoff(self.project_path, &self.mission_id, handoff)?;

        let new_feature_status = if handoff.ok {
            FeatureStatus::Completed
        } else {
            FeatureStatus::Failed
        };
        self.update_feature_status(&handoff.feature_id, new_feature_status)?;

        let mut state = artifacts::read_state(self.project_path, &self.mission_id)?;
        state.assignments.remove(&handoff.worker_id);

        if state.current_worker_id.as_deref() == Some(handoff.worker_id.as_str()) {
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

        state.state = if has_pending {
            if has_running {
                MissionState::Running
            } else {
                MissionState::OrchestratorTurn
            }
        } else if has_running {
            MissionState::Running
        } else {
            MissionState::Completed
        };

        artifacts::write_state(self.project_path, &self.mission_id, &state)?;

        tracing::info!(
            target: "mission",
            mission_id = %self.mission_id,
            feature_id = %handoff.feature_id,
            worker_id = %handoff.worker_id,
            ok = handoff.ok,
            next_state = ?state.state,
            "feature completed"
        );

        Ok(state.state)
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
///   initializing      → running | paused
///   running           → paused | orchestrator_turn | completed
///   paused            → running | completed
///   orchestrator_turn → running | completed | paused
fn validate_transition(from: &MissionState, to: &MissionState) -> Result<(), AppError> {
    let valid = matches!(
        (from, to),
        (MissionState::AwaitingInput, MissionState::Initializing)
            | (MissionState::Initializing, MissionState::Running)
            | (MissionState::Initializing, MissionState::Paused)
            | (MissionState::Running, MissionState::Paused)
            | (MissionState::Running, MissionState::OrchestratorTurn)
            | (MissionState::Running, MissionState::Completed)
            | (MissionState::Paused, MissionState::Running)
            | (MissionState::Paused, MissionState::Completed)
            | (MissionState::OrchestratorTurn, MissionState::Running)
            | (MissionState::OrchestratorTurn, MissionState::Completed)
            | (MissionState::OrchestratorTurn, MissionState::Paused)
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
    use std::fs;
    use std::path::PathBuf;

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

    #[test]
    fn test_create_mission() {
        let project = temp_project_dir();
        let mission_id =
            Orchestrator::create_mission(&project, "Test", "# Mission", sample_features()).unwrap();
        assert!(mission_id.starts_with("mis_"));

        // Verify files
        assert!(artifacts::features_path(&project, &mission_id).exists());
        assert!(artifacts::state_path(&project, &mission_id).exists());

        let state = artifacts::read_state(&project, &mission_id).unwrap();
        assert_eq!(state.state, MissionState::AwaitingInput);

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
        let mission_id =
            Orchestrator::create_mission(&project, "Life", "# Lifecycle", sample_features())
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
        let handoff = HandoffEntry {
            feature_id: "f1".to_string(),
            worker_id: "wk_001".to_string(),
            ok: true,
            summary: "done".to_string(),
            commands_run: Vec::new(),
            artifacts: Vec::new(),
            issues: Vec::new(),
        };
        let next_state = orch.complete_feature(&handoff).unwrap();
        assert_eq!(next_state, MissionState::OrchestratorTurn);
        assert!(!orch.is_finished().unwrap());

        // Start and complete f2
        orch.transition(MissionState::Running).unwrap();
        orch.start_feature("f2", "wk_002", 0).unwrap();
        let handoff2 = HandoffEntry {
            feature_id: "f2".to_string(),
            worker_id: "wk_002".to_string(),
            ok: true,
            summary: "done".to_string(),
            commands_run: Vec::new(),
            artifacts: Vec::new(),
            issues: Vec::new(),
        };
        let final_state = orch.complete_feature(&handoff2).unwrap();
        assert_eq!(final_state, MissionState::Completed);
        assert!(orch.is_finished().unwrap());

        // Verify handoffs
        let handoffs = artifacts::read_handoffs(&project, &mission_id).unwrap();
        assert_eq!(handoffs.len(), 2);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_feature_not_found() {
        let project = temp_project_dir();
        let mission_id =
            Orchestrator::create_mission(&project, "T", "t", sample_features()).unwrap();
        let orch = Orchestrator::new(&project, mission_id);

        let result = orch.update_feature_status("f_nonexistent", FeatureStatus::Completed);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_failed_feature_advances() {
        let project = temp_project_dir();
        let mission_id =
            Orchestrator::create_mission(&project, "T", "t", sample_features()).unwrap();
        let orch = Orchestrator::new(&project, mission_id.clone());

        orch.transition(MissionState::Initializing).unwrap();
        orch.start_feature("f1", "wk_1", 0).unwrap();

        let handoff = HandoffEntry {
            feature_id: "f1".to_string(),
            worker_id: "wk_1".to_string(),
            ok: false,
            summary: "crashed".to_string(),
            commands_run: Vec::new(),
            artifacts: Vec::new(),
            issues: vec!["timeout".to_string()],
        };
        let next = orch.complete_feature(&handoff).unwrap();
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
