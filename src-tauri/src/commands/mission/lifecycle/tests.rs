use super::*;

use crate::mission::artifacts;
use crate::mission::blockers::WorkflowBlockersDoc;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::types::*;
use crate::mission::workflow_types::{
    MissionWorkflowKind, SummaryJobPolicy, WorkflowCreationReason, WorkflowDoc, WorkflowStatus,
};
use crate::review::types::ReviewDecisionRequest;
use tempfile::tempdir;

fn init_mission(project_path: &std::path::Path, mission_id: &str, features: Vec<Feature>) {
    let features_doc = FeaturesDoc::new(mission_id.to_string(), "Test".to_string(), features);
    let mut state_doc = StateDoc::new(
        mission_id.to_string(),
        project_path.to_string_lossy().to_string(),
    );
    state_doc.state = MissionState::Paused;
    let workflow_doc = WorkflowDoc::new(
        mission_id.to_string(),
        MissionWorkflowKind::AdHoc,
        WorkflowCreationReason::ExplicitMissionRequest,
        SummaryJobPolicy::ParentSessionSummary,
        WorkflowStatus::Paused,
    );
    let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
    artifacts::init_mission_dir(
        project_path,
        mission_id,
        "test",
        &features_doc,
        &state_doc,
        &workflow_doc,
        &blockers_doc,
    )
    .unwrap();
}

fn write_running_state(
    project_path: &std::path::Path,
    mission_id: &str,
    worker_id: &str,
    feature_id: &str,
) {
    let now = chrono::Utc::now().timestamp_millis();
    let mut state = artifacts::read_state(project_path, mission_id).unwrap();
    state.state = MissionState::Running;
    state.current_worker_id = Some(worker_id.to_string());
    state.current_feature_id = Some(feature_id.to_string());
    state.assignments.insert(
        worker_id.to_string(),
        WorkerAssignment {
            feature_id: feature_id.to_string(),
            attempt: 1,
            started_at: now,
            last_heartbeat_at: now,
        },
    );
    state.worker_pids.insert(worker_id.to_string(), 12345);
    artifacts::write_state(project_path, mission_id, &state).unwrap();
}

fn recovery_messages(project_path: &std::path::Path, mission_id: &str) -> Vec<String> {
    super::super::runtime::read_mission_recovery_log(project_path, mission_id)
        .into_iter()
        .map(|entry| entry.message)
        .collect()
}

fn test_start_config() -> super::super::MissionStartConfig {
    super::super::MissionStartConfig {
        run_config: super::super::MissionRunConfig {
            model: "test".to_string(),
            provider: "test".to_string(),
            base_url: "http://localhost".to_string(),
            api_key: "test".to_string(),
        },
        max_workers: 1,
        parent_session_id: None,
        parent_turn_id: None,
        delegate_transport: super::super::DelegateTransportMode::Process,
    }
}

#[test]
fn interrupt_rolls_back_in_progress_and_pauses() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_interrupt";

    init_mission(
        project_path,
        mission_id,
        vec![
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::InProgress,
                description: "feature 1".to_string(),
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
                description: "feature 2".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: Vec::new(),
            },
        ],
    );
    write_running_state(project_path, mission_id, "wk_1", "f1");

    let orch = Orchestrator::new(project_path, mission_id.to_string());

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(interrupt_mission(&orch, None, project_path, mission_id))
        .unwrap();

    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::Paused);
    assert!(state.assignments.is_empty());
    assert!(state.worker_pids.is_empty());
    assert!(state.current_feature_id.is_none());

    let features = artifacts::read_features(project_path, mission_id).unwrap();
    let f1 = features.features.iter().find(|f| f.id == "f1").unwrap();
    let f2 = features.features.iter().find(|f| f.id == "f2").unwrap();
    assert_eq!(f1.status, FeatureStatus::Pending);
    assert_eq!(f2.status, FeatureStatus::Pending);

    let recovery = recovery_messages(project_path, mission_id);
    assert!(recovery
        .iter()
        .any(|msg| msg == "mission interrupted by user"));
}

#[test]
fn interrupt_rejects_non_running_mission_without_recovery_log() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_interrupt_not_running";

    init_mission(
        project_path,
        mission_id,
        vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "feature 1".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }],
    );

    let orch = Orchestrator::new(project_path, mission_id.to_string());

    let rt = tokio::runtime::Runtime::new().unwrap();
    let err = rt
        .block_on(interrupt_mission(&orch, None, project_path, mission_id))
        .unwrap_err();

    assert!(err.message.contains("mission is not running"));

    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::Paused);
    assert!(recovery_messages(project_path, mission_id).is_empty());
}

#[test]
fn recover_clears_fake_running_and_rolls_back_feature() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_recover";

    init_mission(
        project_path,
        mission_id,
        vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::InProgress,
            description: "feature 1".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }],
    );
    write_running_state(project_path, mission_id, "wk_1", "f1");

    let orch = Orchestrator::new(project_path, mission_id.to_string());
    recover_mission(&orch, None, project_path, mission_id).unwrap();

    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::Paused);
    assert!(state.assignments.is_empty());
    assert!(state.worker_pids.is_empty());

    let features = artifacts::read_features(project_path, mission_id).unwrap();
    let f1 = features.features.iter().find(|f| f.id == "f1").unwrap();
    assert_eq!(f1.status, FeatureStatus::Pending);

    let recovery = recovery_messages(project_path, mission_id);
    assert!(recovery
        .iter()
        .any(|msg| msg == "mission recovered from fake running"));
}

#[test]
fn recover_paused_mission_is_noop_without_recovery_log() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_recover_paused";

    init_mission(
        project_path,
        mission_id,
        vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "feature 1".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }],
    );

    let orch = Orchestrator::new(project_path, mission_id.to_string());
    recover_mission(&orch, None, project_path, mission_id).unwrap();

    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::Paused);

    let features = artifacts::read_features(project_path, mission_id).unwrap();
    let f1 = features.features.iter().find(|f| f.id == "f1").unwrap();
    assert_eq!(f1.status, FeatureStatus::Pending);
    assert!(recovery_messages(project_path, mission_id).is_empty());
}

#[test]
fn resume_with_config_blocks_on_pending_review_decision() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_resume_gate";

    init_mission(
        project_path,
        mission_id,
        vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "feature 1".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }],
    );

    let now = chrono::Utc::now().timestamp_millis();
    let pending = ReviewDecisionRequest {
        schema_version: 1,
        review_id: "rev_test".to_string(),
        feature_id: Some("f1".to_string()),
        scope_ref: "chapter:ch_1".to_string(),
        target_refs: None,
        question: "decide".to_string(),
        options: vec!["ok".to_string()],
        context_summary: Vec::new(),
        created_at: now,
    };
    artifacts::write_pending_review_decision(project_path, mission_id, &pending).unwrap();

    let orch = Orchestrator::new(project_path, mission_id.to_string());
    let start_config = test_start_config();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let err = rt
        .block_on(resume_mission_with_config(
            None,
            project_path,
            &project_path.to_string_lossy(),
            mission_id,
            &orch,
            None,
            &start_config,
        ))
        .unwrap_err();

    assert!(err.message.contains("pending review decision"));
}

#[test]
fn resume_with_config_blocks_on_pending_knowledge_decision() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_resume_knowledge_gate";

    init_mission(
        project_path,
        mission_id,
        vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "feature 1".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }],
    );

    let pending = crate::knowledge::types::PendingKnowledgeDecision {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: "bundle_test".to_string(),
        delta_id: "delta_test".to_string(),
        scope_ref: "chapter:ch_1".to_string(),
        conflicts: Vec::new(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    artifacts::write_pending_knowledge_decision(project_path, mission_id, &pending).unwrap();

    let orch = Orchestrator::new(project_path, mission_id.to_string());
    let start_config = test_start_config();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let err = rt
        .block_on(resume_mission_with_config(
            None,
            project_path,
            &project_path.to_string_lossy(),
            mission_id,
            &orch,
            None,
            &start_config,
        ))
        .unwrap_err();

    assert!(err.message.contains("pending knowledge decision"));
}

#[test]
fn pending_review_decision_syncs_blocker_state() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_review_blocker_state";

    init_mission(
        project_path,
        mission_id,
        vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "feature 1".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }],
    );

    let pending = ReviewDecisionRequest {
        schema_version: 1,
        review_id: "rev_block".to_string(),
        feature_id: Some("f1".to_string()),
        scope_ref: "chapter:ch_1".to_string(),
        target_refs: None,
        question: "decide".to_string(),
        options: vec!["ok".to_string()],
        context_summary: Vec::new(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    artifacts::write_pending_review_decision(project_path, mission_id, &pending).unwrap();

    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::WaitingReview);
    let blockers = artifacts::read_workflow_blockers(project_path, mission_id).unwrap();
    assert_eq!(blockers.blockers.len(), 1);

    artifacts::clear_pending_review_decision(project_path, mission_id).unwrap();
    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::Paused);
    let blockers = artifacts::read_workflow_blockers(project_path, mission_id).unwrap();
    assert!(blockers.blockers.is_empty());
}

#[test]
fn pending_knowledge_decision_syncs_blocker_state() {
    let dir = tempdir().unwrap();
    let project_path = dir.path();
    let mission_id = "mis_knowledge_blocker_state";

    init_mission(
        project_path,
        mission_id,
        vec![Feature {
            id: "f1".to_string(),
            status: FeatureStatus::Pending,
            description: "feature 1".to_string(),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }],
    );

    let pending = crate::knowledge::types::PendingKnowledgeDecision {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id: "bundle_block".to_string(),
        delta_id: "delta_block".to_string(),
        scope_ref: "chapter:ch_1".to_string(),
        conflicts: Vec::new(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    artifacts::write_pending_knowledge_decision(project_path, mission_id, &pending).unwrap();

    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::WaitingKnowledgeDecision);
    let blockers = artifacts::read_workflow_blockers(project_path, mission_id).unwrap();
    assert_eq!(blockers.blockers.len(), 1);

    artifacts::clear_pending_knowledge_decision(project_path, mission_id).unwrap();
    let state = artifacts::read_state(project_path, mission_id).unwrap();
    assert_eq!(state.state, MissionState::Paused);
    let blockers = artifacts::read_workflow_blockers(project_path, mission_id).unwrap();
    assert!(blockers.blockers.is_empty());
}
