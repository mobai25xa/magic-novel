//! Mission system - Shared data types
//!
//! Pure domain types for features.json, state.json, handoffs.jsonl.
//! Zero dependency on Tauri or agent_engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MISSION_SCHEMA_VERSION: i32 = 1;
pub const MISSION_STATE_SCHEMA_VERSION: i32 = 2;
pub const INTEGRATOR_FEATURE_ID: &str = "__integrator__";

// ── Feature Status ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Failed,
}

// ── Feature ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub status: FeatureStatus,
    pub description: String,
    #[serde(default)]
    pub skill: String,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub expected_behavior: Vec<String>,
    #[serde(default)]
    pub verification_steps: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
}

// ── FeaturesDoc (features.json) ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesDoc {
    pub schema_version: i32,
    pub mission_id: String,
    pub title: String,
    pub features: Vec<Feature>,
}

impl FeaturesDoc {
    pub fn new(mission_id: String, title: String, features: Vec<Feature>) -> Self {
        Self {
            schema_version: MISSION_SCHEMA_VERSION,
            mission_id,
            title,
            features,
        }
    }
}

// ── Mission State Machine ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MissionState {
    AwaitingInput,
    Initializing,
    Running,
    Paused,
    OrchestratorTurn,
    Completed,
}

// ── StateDoc (state.json) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAssignment {
    pub feature_id: String,
    #[serde(default)]
    pub attempt: u32,
    pub started_at: i64,
    pub last_heartbeat_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDoc {
    pub schema_version: i32,
    pub mission_id: String,
    pub state: MissionState,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_feature_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_worker_id: Option<String>,
    #[serde(default)]
    pub assignments: HashMap<String, WorkerAssignment>,
    #[serde(default)]
    pub worker_pids: HashMap<String, u32>,
    pub updated_at: i64,
}

impl StateDoc {
    pub fn new(mission_id: String, cwd: String) -> Self {
        Self {
            schema_version: MISSION_STATE_SCHEMA_VERSION,
            mission_id,
            state: MissionState::AwaitingInput,
            cwd,
            current_feature_id: None,
            current_worker_id: None,
            assignments: HashMap::new(),
            worker_pids: HashMap::new(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

// ── Handoff (handoffs.jsonl, one per line) ──────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffEntry {
    pub feature_id: String,
    pub worker_id: String,
    pub ok: bool,
    pub summary: String,
    #[serde(default)]
    pub commands_run: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub issues: Vec<String>,
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_status_serde() {
        let json = serde_json::to_string(&FeatureStatus::InProgress).unwrap();
        assert_eq!(json, "\"in_progress\"");
        let parsed: FeatureStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, FeatureStatus::InProgress);
    }

    #[test]
    fn test_mission_state_serde() {
        let json = serde_json::to_string(&MissionState::AwaitingInput).unwrap();
        assert_eq!(json, "\"awaiting_input\"");
        let parsed: MissionState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, MissionState::AwaitingInput);
    }

    #[test]
    fn test_features_doc_new() {
        let doc = FeaturesDoc::new(
            "mis_test".to_string(),
            "Test Mission".to_string(),
            vec![Feature {
                id: "f1".to_string(),
                status: FeatureStatus::Pending,
                description: "Do something".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: vec!["It works".to_string()],
                verification_steps: Vec::new(),
                write_paths: Vec::new(),
            }],
        );
        assert_eq!(doc.schema_version, MISSION_SCHEMA_VERSION);
        assert_eq!(doc.features.len(), 1);
        assert_eq!(doc.features[0].id, "f1");
    }

    #[test]
    fn test_state_doc_new() {
        let doc = StateDoc::new("mis_test".to_string(), "/tmp/project".to_string());
        assert_eq!(doc.state, MissionState::AwaitingInput);
        assert_eq!(doc.schema_version, MISSION_STATE_SCHEMA_VERSION);
        assert!(doc.current_feature_id.is_none());
        assert!(doc.assignments.is_empty());
        assert!(doc.worker_pids.is_empty());
    }

    #[test]
    fn test_features_doc_roundtrip() {
        let doc = FeaturesDoc::new(
            "mis_123".to_string(),
            "My Mission".to_string(),
            vec![
                Feature {
                    id: "f1".to_string(),
                    status: FeatureStatus::Pending,
                    description: "First feature".to_string(),
                    skill: "story-architect".to_string(),
                    preconditions: vec!["world built".to_string()],
                    depends_on: Vec::new(),
                    expected_behavior: vec!["chapter created".to_string()],
                    verification_steps: vec!["check chapter".to_string()],
                    write_paths: vec!["manuscripts/ch1.md".to_string()],
                },
                Feature {
                    id: "f2".to_string(),
                    status: FeatureStatus::Completed,
                    description: "Second feature".to_string(),
                    skill: String::new(),
                    preconditions: Vec::new(),
                    depends_on: Vec::new(),
                    expected_behavior: Vec::new(),
                    verification_steps: Vec::new(),
                    write_paths: Vec::new(),
                },
            ],
        );

        let json = serde_json::to_string_pretty(&doc).unwrap();
        let parsed: FeaturesDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mission_id, "mis_123");
        assert_eq!(parsed.features.len(), 2);
        assert_eq!(parsed.features[0].status, FeatureStatus::Pending);
        assert_eq!(parsed.features[1].status, FeatureStatus::Completed);
    }

    #[test]
    fn test_worker_assignment_roundtrip() {
        let now = chrono::Utc::now().timestamp_millis();
        let assignment = WorkerAssignment {
            feature_id: "f1".to_string(),
            attempt: 2,
            started_at: now,
            last_heartbeat_at: now,
        };

        let mut doc = StateDoc::new("mis_test".to_string(), "/tmp/project".to_string());
        doc.assignments
            .insert("wk_1".to_string(), assignment.clone());

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: StateDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.assignments.len(), 1);
        let restored = parsed.assignments.get("wk_1").unwrap();
        assert_eq!(restored.feature_id, assignment.feature_id);
        assert_eq!(restored.attempt, assignment.attempt);
    }

    #[test]
    fn test_handoff_entry_roundtrip() {
        let entry = HandoffEntry {
            feature_id: "f1".to_string(),
            worker_id: "wk_abc".to_string(),
            ok: true,
            summary: "Feature completed successfully".to_string(),
            commands_run: vec!["read ch1".to_string(), "edit ch1".to_string()],
            artifacts: vec!["ch1.md".to_string()],
            issues: Vec::new(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: HandoffEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.feature_id, "f1");
        assert!(parsed.ok);
        assert_eq!(parsed.commands_run.len(), 2);
    }

    #[test]
    fn test_handoff_entry_minimal_json() {
        // Ensure default fields work when not present in JSON
        let json = r#"{"feature_id":"f1","worker_id":"wk_1","ok":false,"summary":"err"}"#;
        let parsed: HandoffEntry = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.feature_id, "f1");
        assert!(!parsed.ok);
        assert!(parsed.commands_run.is_empty());
        assert!(parsed.artifacts.is_empty());
        assert!(parsed.issues.is_empty());
    }
}
