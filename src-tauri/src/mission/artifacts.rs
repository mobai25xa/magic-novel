//! Mission system - Artifact I/O (on-disk mission directory management)
//!
//! Directory structure: {project_path}/magic_novel/missions/{mission_id}/
//!   mission.md       -- user-provided mission description
//!   features.json    -- FeaturesDoc (atomic write)
//!   state.json       -- StateDoc (atomic write)
//!   handoffs.jsonl   -- append-only HandoffEntry lines

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::contextpack_types::ContextPack;
use super::layer1_types::{ActiveCast, ChapterCard, Layer1Snapshot, RecentFacts};

use crate::knowledge::types::{KnowledgeDelta, KnowledgeProposalBundle, PendingKnowledgeDecision};
use crate::models::AppError;
use crate::review::types::{ReviewDecisionRequest, ReviewReport};
use crate::utils::atomic_write::atomic_write_json;

use super::types::*;
use super::worker_profile::WorkerRunEntry;

// ── Path helpers ────────────────────────────────────────────────

pub const MAGIC_NOVEL_DIR: &str = "magic_novel";
pub const MISSIONS_DIR: &str = "missions";

pub fn missions_root(project_path: &Path) -> PathBuf {
    project_path.join(MAGIC_NOVEL_DIR).join(MISSIONS_DIR)
}

pub fn mission_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    missions_root(project_path).join(mission_id)
}

pub fn mission_md_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("mission.md")
}

pub fn features_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("features.json")
}

pub fn state_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("state.json")
}

pub fn handoffs_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("handoffs.jsonl")
}

pub fn worker_runs_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("worker_runs.jsonl")
}

pub const LAYER1_DIR: &str = "layer1";
pub const CONTEXTPACKS_DIR: &str = "contextpacks";
pub const REVIEWS_DIR: &str = "reviews";
pub const KNOWLEDGE_DIR: &str = "knowledge";
pub const KNOWLEDGE_BUNDLES_DIR: &str = "bundles";
pub const KNOWLEDGE_DELTAS_DIR: &str = "deltas";

pub fn layer1_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join(LAYER1_DIR)
}

pub fn contextpacks_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join(CONTEXTPACKS_DIR)
}

pub fn reviews_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join(REVIEWS_DIR)
}

pub fn knowledge_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join(KNOWLEDGE_DIR)
}

pub fn knowledge_bundles_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    knowledge_dir(project_path, mission_id).join(KNOWLEDGE_BUNDLES_DIR)
}

pub fn knowledge_deltas_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    knowledge_dir(project_path, mission_id).join(KNOWLEDGE_DELTAS_DIR)
}

pub fn layer1_chapter_card_path(project_path: &Path, mission_id: &str) -> PathBuf {
    layer1_dir(project_path, mission_id).join("chapter_card.json")
}

pub fn layer1_recent_facts_path(project_path: &Path, mission_id: &str) -> PathBuf {
    layer1_dir(project_path, mission_id).join("recent_facts.json")
}

pub fn layer1_active_cast_path(project_path: &Path, mission_id: &str) -> PathBuf {
    layer1_dir(project_path, mission_id).join("active_cast.json")
}

pub fn layer1_active_foreshadowing_path(project_path: &Path, mission_id: &str) -> PathBuf {
    layer1_dir(project_path, mission_id).join("active_foreshadowing.json")
}

pub fn layer1_previous_summary_path(project_path: &Path, mission_id: &str) -> PathBuf {
    layer1_dir(project_path, mission_id).join("previous_summary.json")
}

pub fn layer1_risk_ledger_path(project_path: &Path, mission_id: &str) -> PathBuf {
    layer1_dir(project_path, mission_id).join("risk_ledger.json")
}

pub fn latest_contextpack_path(project_path: &Path, mission_id: &str) -> PathBuf {
    contextpacks_dir(project_path, mission_id).join("contextpack.json")
}

pub fn review_latest_path(project_path: &Path, mission_id: &str) -> PathBuf {
    reviews_dir(project_path, mission_id).join("latest.json")
}

pub fn review_reports_path(project_path: &Path, mission_id: &str) -> PathBuf {
    reviews_dir(project_path, mission_id).join("reports.jsonl")
}

pub fn pending_review_decision_path(project_path: &Path, mission_id: &str) -> PathBuf {
    reviews_dir(project_path, mission_id).join("pending_decision.json")
}

pub fn knowledge_bundle_latest_path(project_path: &Path, mission_id: &str) -> PathBuf {
    knowledge_bundles_dir(project_path, mission_id).join("latest.json")
}

pub fn knowledge_bundles_path(project_path: &Path, mission_id: &str) -> PathBuf {
    knowledge_bundles_dir(project_path, mission_id).join("bundles.jsonl")
}

pub fn knowledge_delta_latest_path(project_path: &Path, mission_id: &str) -> PathBuf {
    knowledge_deltas_dir(project_path, mission_id).join("latest.json")
}

pub fn knowledge_deltas_path(project_path: &Path, mission_id: &str) -> PathBuf {
    knowledge_deltas_dir(project_path, mission_id).join("deltas.jsonl")
}

pub fn pending_knowledge_decision_path(project_path: &Path, mission_id: &str) -> PathBuf {
    knowledge_dir(project_path, mission_id).join("pending_decision.json")
}

// ── Init ────────────────────────────────────────────────────────

/// Create the mission directory and write initial artifacts.
pub fn init_mission_dir(
    project_path: &Path,
    mission_id: &str,
    mission_text: &str,
    features_doc: &FeaturesDoc,
    state_doc: &StateDoc,
) -> Result<PathBuf, AppError> {
    let dir = mission_dir(project_path, mission_id);
    std::fs::create_dir_all(&dir)?;

    // Write mission.md (plain text)
    std::fs::write(mission_md_path(project_path, mission_id), mission_text)?;

    // Write features.json (atomic)
    atomic_write_json(&features_path(project_path, mission_id), features_doc)?;

    // Write state.json (atomic)
    atomic_write_json(&state_path(project_path, mission_id), state_doc)?;

    // Create empty handoffs.jsonl
    std::fs::write(handoffs_path(project_path, mission_id), "")?;

    // Create empty worker_runs.jsonl (append-only)
    std::fs::write(worker_runs_path(project_path, mission_id), "")?;

    Ok(dir)
}

// ── Read ────────────────────────────────────────────────────────

pub fn read_features(project_path: &Path, mission_id: &str) -> Result<FeaturesDoc, AppError> {
    let path = features_path(project_path, mission_id);
    let content = std::fs::read_to_string(&path)?;
    let doc: FeaturesDoc = serde_json::from_str(&content)?;
    Ok(doc)
}

pub fn read_state(project_path: &Path, mission_id: &str) -> Result<StateDoc, AppError> {
    #[derive(Debug, Deserialize)]
    struct LegacyStateDoc {
        #[serde(rename = "schema_version")]
        _schema_version: i32,
        mission_id: String,
        state: MissionState,
        cwd: String,
        #[serde(default)]
        current_feature_id: Option<String>,
        #[serde(default)]
        current_worker_id: Option<String>,
        #[serde(default)]
        worker_pids: std::collections::HashMap<String, u32>,
        updated_at: i64,
    }

    let path = state_path(project_path, mission_id);
    let content = std::fs::read_to_string(&path)?;

    let raw_value: serde_json::Value = serde_json::from_str(&content)?;
    let schema_version = raw_value
        .get("schema_version")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    if schema_version >= MISSION_STATE_SCHEMA_VERSION as i64 {
        let mut doc: StateDoc = serde_json::from_value(raw_value)?;
        doc.schema_version = MISSION_STATE_SCHEMA_VERSION;
        if doc.current_worker_id.is_none() || doc.current_feature_id.is_none() {
            if let Some((worker_id, assignment)) = doc
                .assignments
                .iter()
                .max_by_key(|(_, assignment)| assignment.started_at)
            {
                doc.current_worker_id = Some(worker_id.clone());
                doc.current_feature_id = Some(assignment.feature_id.clone());
            }
        }
        return Ok(doc);
    }

    let legacy: LegacyStateDoc = serde_json::from_value(raw_value)?;
    let mut assignments = std::collections::HashMap::new();
    if let (Some(worker_id), Some(feature_id)) = (
        legacy.current_worker_id.clone(),
        legacy.current_feature_id.clone(),
    ) {
        assignments.insert(
            worker_id,
            WorkerAssignment {
                feature_id,
                attempt: 0,
                started_at: legacy.updated_at,
                last_heartbeat_at: legacy.updated_at,
            },
        );
    }

    Ok(StateDoc {
        schema_version: MISSION_STATE_SCHEMA_VERSION,
        mission_id: legacy.mission_id,
        state: legacy.state,
        cwd: legacy.cwd,
        current_feature_id: legacy.current_feature_id,
        current_worker_id: legacy.current_worker_id,
        assignments,
        worker_pids: legacy.worker_pids,
        updated_at: legacy.updated_at,
    })
}

pub fn read_mission_md(project_path: &Path, mission_id: &str) -> Result<String, AppError> {
    let path = mission_md_path(project_path, mission_id);
    Ok(std::fs::read_to_string(&path)?)
}

pub fn read_handoffs(project_path: &Path, mission_id: &str) -> Result<Vec<HandoffEntry>, AppError> {
    let path = handoffs_path(project_path, mission_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let entries: Vec<HandoffEntry> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            serde_json::from_str(l)
                .map_err(
                    |e| tracing::warn!(target: "mission", line = %l, "handoff parse error: {e}"),
                )
                .ok()
        })
        .collect();
    Ok(entries)
}

fn read_optional_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let doc: T = serde_json::from_str(&content)?;
    Ok(Some(doc))
}

// ── Read: Layer1 / ContextPack (M2) ─────────────────────────────

pub fn read_layer1_chapter_card(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<ChapterCard>, AppError> {
    read_optional_json(&layer1_chapter_card_path(project_path, mission_id))
}

pub fn read_layer1_recent_facts(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<RecentFacts>, AppError> {
    read_optional_json(&layer1_recent_facts_path(project_path, mission_id))
}

pub fn read_layer1_active_cast(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<ActiveCast>, AppError> {
    read_optional_json(&layer1_active_cast_path(project_path, mission_id))
}

pub fn read_layer1_snapshot(project_path: &Path, mission_id: &str) -> Result<Layer1Snapshot, AppError> {
    Ok(Layer1Snapshot {
        chapter_card: read_layer1_chapter_card(project_path, mission_id)?,
        recent_facts: read_layer1_recent_facts(project_path, mission_id)?,
        active_cast: read_layer1_active_cast(project_path, mission_id)?,
        active_foreshadowing: read_optional_json(&layer1_active_foreshadowing_path(project_path, mission_id))?,
        previous_summary: read_optional_json(&layer1_previous_summary_path(project_path, mission_id))?,
        risk_ledger: read_optional_json(&layer1_risk_ledger_path(project_path, mission_id))?,
    })
}

pub fn read_latest_contextpack(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<ContextPack>, AppError> {
    read_optional_json(&latest_contextpack_path(project_path, mission_id))
}

// ── Read: Reviews (M3) ────────────────────────────────────────

pub fn read_review_latest(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<ReviewReport>, AppError> {
    read_optional_json(&review_latest_path(project_path, mission_id))
}

pub fn read_review_reports(
    project_path: &Path,
    mission_id: &str,
) -> Result<Vec<ReviewReport>, AppError> {
    let path = review_reports_path(project_path, mission_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let entries: Vec<ReviewReport> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            serde_json::from_str(l)
                .map_err(|e| tracing::warn!(target: "mission", line = %l, "review report parse error: {e}"))
                .ok()
        })
        .collect();
    Ok(entries)
}

pub fn read_pending_review_decision(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<ReviewDecisionRequest>, AppError> {
    read_optional_json(&pending_review_decision_path(project_path, mission_id))
}

// ── Read: Knowledge writeback (M4) ────────────────────────────

pub fn read_knowledge_bundle_latest(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<KnowledgeProposalBundle>, AppError> {
    read_optional_json(&knowledge_bundle_latest_path(project_path, mission_id))
}

pub fn read_knowledge_delta_latest(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<KnowledgeDelta>, AppError> {
    read_optional_json(&knowledge_delta_latest_path(project_path, mission_id))
}

pub fn read_pending_knowledge_decision(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<PendingKnowledgeDecision>, AppError> {
    read_optional_json(&pending_knowledge_decision_path(project_path, mission_id))
}

// ── Write (update) ──────────────────────────────────────────────

pub fn write_features(
    project_path: &Path,
    mission_id: &str,
    doc: &FeaturesDoc,
) -> Result<(), AppError> {
    atomic_write_json(&features_path(project_path, mission_id), doc)
}

pub fn write_state(project_path: &Path, mission_id: &str, doc: &StateDoc) -> Result<(), AppError> {
    atomic_write_json(&state_path(project_path, mission_id), doc)
}

pub fn append_handoff(
    project_path: &Path,
    mission_id: &str,
    entry: &HandoffEntry,
) -> Result<(), AppError> {
    let path = handoffs_path(project_path, mission_id);
    let line = serde_json::to_string(entry)? + "\n";
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(())
}

pub fn append_worker_run(
    project_path: &Path,
    mission_id: &str,
    entry: &WorkerRunEntry,
) -> Result<(), AppError> {
    let path = worker_runs_path(project_path, mission_id);
    let line = serde_json::to_string(entry)? + "\n";
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(())
}

// ── Write: Layer1 / ContextPack (M2) ────────────────────────────

fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn write_layer1_chapter_card(
    project_path: &Path,
    mission_id: &str,
    doc: &ChapterCard,
) -> Result<(), AppError> {
    let path = layer1_chapter_card_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn write_layer1_recent_facts(
    project_path: &Path,
    mission_id: &str,
    doc: &RecentFacts,
) -> Result<(), AppError> {
    let path = layer1_recent_facts_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn write_layer1_active_cast(
    project_path: &Path,
    mission_id: &str,
    doc: &ActiveCast,
) -> Result<(), AppError> {
    let path = layer1_active_cast_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn write_latest_contextpack(
    project_path: &Path,
    mission_id: &str,
    doc: &ContextPack,
) -> Result<(), AppError> {
    let path = latest_contextpack_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

// ── Write: Reviews (M3) ───────────────────────────────────────

pub fn write_review_latest(
    project_path: &Path,
    mission_id: &str,
    doc: &ReviewReport,
) -> Result<(), AppError> {
    let path = review_latest_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn append_review_report(
    project_path: &Path,
    mission_id: &str,
    entry: &ReviewReport,
) -> Result<(), AppError> {
    let path = review_reports_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    let line = serde_json::to_string(entry)? + "\n";
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(())
}

pub fn write_pending_review_decision(
    project_path: &Path,
    mission_id: &str,
    doc: &ReviewDecisionRequest,
) -> Result<(), AppError> {
    let path = pending_review_decision_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn clear_pending_review_decision(project_path: &Path, mission_id: &str) -> Result<(), AppError> {
    let path = pending_review_decision_path(project_path, mission_id);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

// ── Write: Knowledge writeback (M4) ───────────────────────────

pub fn write_knowledge_bundle_latest(
    project_path: &Path,
    mission_id: &str,
    doc: &KnowledgeProposalBundle,
) -> Result<(), AppError> {
    let path = knowledge_bundle_latest_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn append_knowledge_bundle(
    project_path: &Path,
    mission_id: &str,
    entry: &KnowledgeProposalBundle,
) -> Result<(), AppError> {
    let path = knowledge_bundles_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    let line = serde_json::to_string(entry)? + "\n";
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(())
}

pub fn write_knowledge_delta_latest(
    project_path: &Path,
    mission_id: &str,
    doc: &KnowledgeDelta,
) -> Result<(), AppError> {
    let path = knowledge_delta_latest_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn append_knowledge_delta(
    project_path: &Path,
    mission_id: &str,
    entry: &KnowledgeDelta,
) -> Result<(), AppError> {
    let path = knowledge_deltas_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    let line = serde_json::to_string(entry)? + "\n";
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    Ok(())
}

pub fn write_pending_knowledge_decision(
    project_path: &Path,
    mission_id: &str,
    doc: &PendingKnowledgeDecision,
) -> Result<(), AppError> {
    let path = pending_knowledge_decision_path(project_path, mission_id);
    ensure_parent_dir(&path)?;
    atomic_write_json(&path, doc)
}

pub fn clear_pending_knowledge_decision(
    project_path: &Path,
    mission_id: &str,
) -> Result<(), AppError> {
    let path = pending_knowledge_decision_path(project_path, mission_id);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

// ── List missions ───────────────────────────────────────────────

pub fn list_missions(project_path: &Path) -> Result<Vec<String>, AppError> {
    let root = missions_root(project_path);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("mis_") {
                    ids.push(name.to_string());
                }
            }
        }
    }
    ids.sort();
    Ok(ids)
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::mission::contextpack_types::{TokenBudget, CONTEXTPACK_SCHEMA_VERSION};
    use crate::mission::layer1_types::{
        ChapterCardStatus, ChapterWorkflowKind, LAYER1_SCHEMA_VERSION,
    };

    fn temp_project_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("magic_test_{}", uuid::Uuid::new_v4()));
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
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: vec!["chapters/ch2.md".to_string()],
            },
        ]
    }

    #[test]
    fn test_init_and_read_mission() {
        let project = temp_project_dir();
        let mission_id = "mis_test_001";

        let features_doc = FeaturesDoc::new(
            mission_id.to_string(),
            "Test Mission".to_string(),
            sample_features(),
        );
        let state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );

        let dir = init_mission_dir(
            &project,
            mission_id,
            "# My Mission\nGoal: test",
            &features_doc,
            &state_doc,
        )
        .unwrap();
        assert!(dir.exists());

        // Verify files exist
        assert!(mission_md_path(&project, mission_id).exists());
        assert!(features_path(&project, mission_id).exists());
        assert!(state_path(&project, mission_id).exists());
        assert!(handoffs_path(&project, mission_id).exists());
        assert!(worker_runs_path(&project, mission_id).exists());

        // Read back
        let md = read_mission_md(&project, mission_id).unwrap();
        assert!(md.contains("My Mission"));

        let features = read_features(&project, mission_id).unwrap();
        assert_eq!(features.features.len(), 2);
        assert_eq!(features.features[0].id, "f1");

        let state = read_state(&project, mission_id).unwrap();
        assert_eq!(state.state, MissionState::AwaitingInput);

        let handoffs = read_handoffs(&project, mission_id).unwrap();
        assert!(handoffs.is_empty());

        // Cleanup
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_write_and_read_features() {
        let project = temp_project_dir();
        let mission_id = "mis_test_002";

        let mut features_doc = FeaturesDoc::new(
            mission_id.to_string(),
            "Test".to_string(),
            sample_features(),
        );
        let state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        init_mission_dir(&project, mission_id, "test", &features_doc, &state_doc).unwrap();

        // Update features
        features_doc.features[0].status = FeatureStatus::InProgress;
        write_features(&project, mission_id, &features_doc).unwrap();

        let read_back = read_features(&project, mission_id).unwrap();
        assert_eq!(read_back.features[0].status, FeatureStatus::InProgress);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_layer1_and_contextpack_io_lazy_create() {
        let project = temp_project_dir();
        let mission_id = "mis_test_layer1";

        let features_doc = FeaturesDoc::new(mission_id.to_string(), "T".to_string(), Vec::new());
        let state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        init_mission_dir(&project, mission_id, "t", &features_doc, &state_doc).unwrap();

        // Layer1 dirs should be lazy: write should create parents.
        let cc = ChapterCard {
            schema_version: LAYER1_SCHEMA_VERSION,
            scope_ref: "chapter:ch_1".to_string(),
            scope_locator: Some("vol1/ch1.json".to_string()),
            objective: "Test objective".to_string(),
            workflow_kind: ChapterWorkflowKind::Chapter,
            hard_constraints: vec!["Keep tense".to_string()],
            success_criteria: vec!["Sounds good".to_string()],
            status: ChapterCardStatus::Active,
            updated_at: 1,
        };
        write_layer1_chapter_card(&project, mission_id, &cc).unwrap();
        assert!(layer1_chapter_card_path(&project, mission_id).exists());

        let snap = read_layer1_snapshot(&project, mission_id).unwrap();
        assert!(snap.chapter_card.is_some());
        assert!(snap.recent_facts.is_none());

        let mut cp = ContextPack::default();
        cp.schema_version = CONTEXTPACK_SCHEMA_VERSION;
        cp.scope_ref = "chapter:ch_1".to_string();
        cp.token_budget = TokenBudget::Small;
        cp.generated_at = 2;
        write_latest_contextpack(&project, mission_id, &cp).unwrap();
        assert!(latest_contextpack_path(&project, mission_id).exists());

        let loaded = read_latest_contextpack(&project, mission_id).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().scope_ref, "chapter:ch_1");

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_read_state_migrates_v1() {
        let project = temp_project_dir();
        let mission_id = "mis_test_v1";
        let mission_root = mission_dir(&project, mission_id);
        fs::create_dir_all(&mission_root).unwrap();

        let legacy_state = serde_json::json!({
            "schema_version": 1,
            "mission_id": mission_id,
            "state": "running",
            "cwd": project.to_string_lossy().to_string(),
            "current_feature_id": "f1",
            "current_worker_id": "wk_legacy",
            "worker_pids": {"wk_legacy": 12345},
            "updated_at": 1700000000000_i64
        });

        fs::write(
            state_path(&project, mission_id),
            serde_json::to_string_pretty(&legacy_state).unwrap(),
        )
        .unwrap();

        let migrated = read_state(&project, mission_id).unwrap();
        assert_eq!(migrated.schema_version, MISSION_STATE_SCHEMA_VERSION);
        assert_eq!(migrated.current_feature_id.as_deref(), Some("f1"));
        assert_eq!(migrated.current_worker_id.as_deref(), Some("wk_legacy"));
        assert_eq!(migrated.assignments.len(), 1);
        assert!(migrated.assignments.contains_key("wk_legacy"));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_append_and_read_handoffs() {
        let project = temp_project_dir();
        let mission_id = "mis_test_003";

        let features_doc = FeaturesDoc::new(mission_id.to_string(), "T".to_string(), Vec::new());
        let state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        init_mission_dir(&project, mission_id, "t", &features_doc, &state_doc).unwrap();

        let h1 = HandoffEntry {
            feature_id: "f1".to_string(),
            worker_id: "wk_1".to_string(),
            ok: true,
            summary: "done".to_string(),
            commands_run: Vec::new(),
            artifacts: Vec::new(),
            issues: Vec::new(),
        };
        let h2 = HandoffEntry {
            feature_id: "f2".to_string(),
            worker_id: "wk_2".to_string(),
            ok: false,
            summary: "failed".to_string(),
            commands_run: Vec::new(),
            artifacts: Vec::new(),
            issues: vec!["timeout".to_string()],
        };

        append_handoff(&project, mission_id, &h1).unwrap();
        append_handoff(&project, mission_id, &h2).unwrap();

        let handoffs = read_handoffs(&project, mission_id).unwrap();
        assert_eq!(handoffs.len(), 2);
        assert!(handoffs[0].ok);
        assert!(!handoffs[1].ok);
        assert_eq!(handoffs[1].issues[0], "timeout");

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_list_missions() {
        let project = temp_project_dir();

        // No missions yet
        let list = list_missions(&project).unwrap();
        assert!(list.is_empty());

        // Create missions
        let root = missions_root(&project);
        fs::create_dir_all(root.join("mis_aaa")).unwrap();
        fs::create_dir_all(root.join("mis_bbb")).unwrap();
        fs::create_dir_all(root.join("other_dir")).unwrap(); // should be ignored

        let list = list_missions(&project).unwrap();
        assert_eq!(list, vec!["mis_aaa", "mis_bbb"]);

        let _ = fs::remove_dir_all(&project);
    }
}
