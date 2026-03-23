//! Mission system - Artifact I/O (on-disk mission directory management)
//!
//! Directory structure: {project_path}/magic_novel/missions/{mission_id}/
//!   mission.md       -- user-provided mission description
//!   features.json    -- FeaturesDoc (atomic write)
//!   state.json       -- StateDoc (atomic write)
//!   task_results.jsonl -- append-only AgentTaskResult lines
//!   handoffs.jsonl   -- append-only HandoffEntry lines

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::blockers::{derive_blockers, WorkflowBlocker, WorkflowBlockerKind, WorkflowBlockersDoc};
use super::contextpack_types::ContextPack;
use super::delegate_types::DelegateResult;
use super::job_types::{JobSnapshot, JobStatus};
use super::layer1_types::{ActiveCast, ChapterCard, Layer1Snapshot, RecentFacts};
use super::result_types::AgentTaskResult;
use super::workflow_types::{
    MissionWorkflowKind, SummaryJobPolicy, WorkflowCreationReason, WorkflowDoc, WorkflowStatus,
};

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

pub fn workflow_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("workflow.json")
}

pub fn blockers_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("blockers.json")
}

pub fn handoffs_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("handoffs.jsonl")
}

pub fn task_results_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("task_results.jsonl")
}

pub fn worker_runs_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("worker_runs.jsonl")
}

pub fn job_snapshot_path(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join("job_snapshot.json")
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
    workflow_doc: &WorkflowDoc,
    blockers_doc: &WorkflowBlockersDoc,
) -> Result<PathBuf, AppError> {
    let dir = mission_dir(project_path, mission_id);
    std::fs::create_dir_all(&dir)?;

    // Write mission.md (plain text)
    std::fs::write(mission_md_path(project_path, mission_id), mission_text)?;

    // Write features.json (atomic)
    atomic_write_json(&features_path(project_path, mission_id), features_doc)?;

    // Write state.json (atomic)
    atomic_write_json(&state_path(project_path, mission_id), state_doc)?;

    // Write workflow.json and blockers.json (atomic)
    atomic_write_json(&workflow_path(project_path, mission_id), workflow_doc)?;
    atomic_write_json(&blockers_path(project_path, mission_id), blockers_doc)?;

    // Create empty task_results.jsonl and handoffs.jsonl
    std::fs::write(task_results_path(project_path, mission_id), "")?;
    std::fs::write(handoffs_path(project_path, mission_id), "")?;

    // Create empty worker_runs.jsonl (append-only)
    std::fs::write(worker_runs_path(project_path, mission_id), "")?;

    let mut initial_snapshot = JobSnapshot::from_workflow(workflow_doc);
    initial_snapshot.blockers = blockers_doc.blockers.clone();
    initial_snapshot.updated_at = chrono::Utc::now().timestamp_millis();
    atomic_write_json(
        &job_snapshot_path(project_path, mission_id),
        &initial_snapshot,
    )?;

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

pub fn read_workflow(project_path: &Path, mission_id: &str) -> Result<WorkflowDoc, AppError> {
    let path = workflow_path(project_path, mission_id);
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let doc: WorkflowDoc = serde_json::from_str(&content)?;
        return Ok(doc);
    }

    let state = read_state(project_path, mission_id)?;
    let mut doc = WorkflowDoc::new(
        mission_id.to_string(),
        MissionWorkflowKind::AdHoc,
        WorkflowCreationReason::ExplicitMissionRequest,
        SummaryJobPolicy::ParentSessionSummary,
        WorkflowStatus::from_mission_state(&state.state),
    );
    doc.created_at = state.updated_at;
    doc.updated_at = state.updated_at;
    Ok(doc)
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

pub fn read_task_results(
    project_path: &Path,
    mission_id: &str,
) -> Result<Vec<AgentTaskResult>, AppError> {
    let path = task_results_path(project_path, mission_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let entries: Vec<AgentTaskResult> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            serde_json::from_str(l)
                .map_err(|e| {
                    tracing::warn!(target: "mission", line = %l, "task result parse error: {e}")
                })
                .ok()
        })
        .collect();
    Ok(entries)
}

fn map_task_results_to_delegate_results(
    mission_id: &str,
    task_results: Vec<AgentTaskResult>,
) -> Vec<DelegateResult> {
    task_results
        .into_iter()
        .map(|result| {
            let delegate_id = result.actor_id.clone();
            let parent_task_id = result.task_id.clone();
            DelegateResult::from_agent_task_result(
                delegate_id,
                mission_id.to_string(),
                parent_task_id,
                result,
            )
        })
        .collect()
}

fn task_result_snapshot_key(result: &DelegateResult) -> Option<String> {
    let parent_task_id = result.parent_task_id.trim();
    if !parent_task_id.is_empty() {
        return Some(format!("task:{}", parent_task_id.to_ascii_lowercase()));
    }

    let delegate_id = result.delegate_id.trim();
    if !delegate_id.is_empty() {
        return Some(format!("delegate:{}", delegate_id.to_ascii_lowercase()));
    }

    result
        .actor_id
        .as_deref()
        .map(str::trim)
        .filter(|actor_id| !actor_id.is_empty())
        .map(|actor_id| format!("actor:{}", actor_id.to_ascii_lowercase()))
}

fn aggregate_task_results_for_snapshot(
    mission_id: &str,
    task_results: Vec<AgentTaskResult>,
) -> Vec<DelegateResult> {
    let mut latest_by_key = std::collections::HashMap::<String, (usize, DelegateResult)>::new();
    let mut passthrough = Vec::new();

    for (index, result) in map_task_results_to_delegate_results(mission_id, task_results)
        .into_iter()
        .enumerate()
    {
        if let Some(key) = task_result_snapshot_key(&result) {
            latest_by_key.insert(key, (index, result));
        } else {
            passthrough.push((index, result));
        }
    }

    let mut aggregated = latest_by_key.into_values().collect::<Vec<_>>();
    aggregated.extend(passthrough);
    aggregated.sort_by_key(|(index, _)| *index);
    aggregated.into_iter().map(|(_, result)| result).collect()
}

fn delegate_results_semantically_match(
    left: &[DelegateResult],
    right: &[DelegateResult],
) -> Result<bool, AppError> {
    Ok(serde_json::to_value(left)? == serde_json::to_value(right)?)
}

fn snapshot_semantically_matches(
    snapshot: &JobSnapshot,
    current: &JobSnapshot,
) -> Result<bool, AppError> {
    Ok(snapshot.schema_version == current.schema_version
        && snapshot.job_id == current.job_id
        && snapshot.job_kind == current.job_kind
        && snapshot.status == current.status
        && snapshot.blockers == current.blockers
        && snapshot.ready_tasks == current.ready_tasks
        && snapshot.running_tasks == current.running_tasks
        && snapshot.completed_tasks == current.completed_tasks
        && snapshot.failed_tasks == current.failed_tasks
        && delegate_results_semantically_match(&snapshot.task_results, &current.task_results)?)
}

fn normalized_feature_id(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

fn runtime_blocker_feature_id(state: &StateDoc) -> Option<String> {
    if let Some(feature_id) = normalized_feature_id(state.current_feature_id.clone()) {
        return Some(feature_id);
    }

    if let Some(feature_id) = state
        .current_worker_id
        .as_deref()
        .and_then(|worker_id| state.assignments.get(worker_id))
        .map(|assignment| assignment.feature_id.clone())
        .and_then(|feature_id| normalized_feature_id(Some(feature_id)))
    {
        return Some(feature_id);
    }

    state
        .assignments
        .values()
        .max_by_key(|assignment| assignment.last_heartbeat_at.max(assignment.started_at))
        .map(|assignment| assignment.feature_id.clone())
        .and_then(|feature_id| normalized_feature_id(Some(feature_id)))
}

fn job_status_from_mission_state(state: &MissionState) -> JobStatus {
    JobStatus::from(&WorkflowStatus::from_mission_state(state))
}

fn compute_ready_task_ids(
    features: &[Feature],
    running_task_ids: &std::collections::HashSet<String>,
) -> Vec<String> {
    let status_by_id = features
        .iter()
        .map(|feature| {
            let status = if running_task_ids.contains(&feature.id) {
                FeatureStatus::InProgress
            } else {
                feature.status.clone()
            };
            (feature.id.as_str(), status)
        })
        .collect::<std::collections::HashMap<_, _>>();

    features
        .iter()
        .filter(|feature| feature.status == FeatureStatus::Pending)
        .filter(|feature| {
            feature.depends_on.iter().all(|dep| {
                matches!(
                    status_by_id.get(dep.as_str()),
                    Some(FeatureStatus::Completed)
                )
            })
        })
        .map(|feature| feature.id.clone())
        .collect()
}

fn build_job_snapshot(project_path: &Path, mission_id: &str) -> Result<JobSnapshot, AppError> {
    let workflow = read_workflow(project_path, mission_id)?;
    let features_doc = read_features(project_path, mission_id)?;
    let state_doc = read_state(project_path, mission_id)?;
    let blockers_doc = read_workflow_blockers(project_path, mission_id)?;
    let task_results = read_task_results(project_path, mission_id)?;

    let mut snapshot = JobSnapshot::from_workflow(&workflow);
    snapshot.blockers = blockers_doc.blockers.clone();
    snapshot.status = blocker_state(&snapshot.blockers)
        .map(|state| job_status_from_mission_state(&state))
        .unwrap_or_else(|| job_status_from_mission_state(&state_doc.state));
    snapshot.running_tasks = state_doc
        .assignments
        .values()
        .map(|assignment| assignment.feature_id.clone())
        .collect();
    snapshot.completed_tasks = features_doc
        .features
        .iter()
        .filter(|feature| feature.status == FeatureStatus::Completed)
        .map(|feature| feature.id.clone())
        .collect();
    snapshot.failed_tasks = features_doc
        .features
        .iter()
        .filter(|feature| {
            matches!(
                feature.status,
                FeatureStatus::Failed | FeatureStatus::Cancelled
            )
        })
        .map(|feature| feature.id.clone())
        .collect();

    let running_set = snapshot
        .running_tasks
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    snapshot.ready_tasks = compute_ready_task_ids(&features_doc.features, &running_set);
    snapshot.task_results = aggregate_task_results_for_snapshot(mission_id, task_results);

    snapshot.running_tasks.sort();
    snapshot.running_tasks.dedup();
    snapshot.completed_tasks.sort();
    snapshot.completed_tasks.dedup();
    snapshot.failed_tasks.sort();
    snapshot.failed_tasks.dedup();
    snapshot.ready_tasks.sort();
    snapshot.ready_tasks.dedup();
    snapshot.updated_at = chrono::Utc::now().timestamp_millis();

    Ok(snapshot)
}

pub fn read_job_snapshot(project_path: &Path, mission_id: &str) -> Result<JobSnapshot, AppError> {
    let path = job_snapshot_path(project_path, mission_id);
    if path.exists() {
        match std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str::<JobSnapshot>(&content).ok())
        {
            Some(doc) => {
                let current = build_job_snapshot(project_path, mission_id)?;
                if snapshot_semantically_matches(&doc, &current)? {
                    return Ok(doc);
                }

                tracing::warn!(
                    target: "mission",
                    mission_id = %mission_id,
                    "job snapshot stale; rebuilding from current mission artifacts"
                );
                let _ = write_job_snapshot(project_path, mission_id, &current);
                return Ok(current);
            }
            None => {
                tracing::warn!(
                    target: "mission",
                    mission_id = %mission_id,
                    "job snapshot missing or invalid; rebuilding from workflow/task artifacts"
                );
            }
        }
    }

    let snapshot = build_job_snapshot(project_path, mission_id)?;
    let _ = write_job_snapshot(project_path, mission_id, &snapshot);
    Ok(snapshot)
}

fn read_optional_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let doc: T = serde_json::from_str(&content)?;
    Ok(Some(doc))
}

pub fn read_workflow_blockers(
    project_path: &Path,
    mission_id: &str,
) -> Result<WorkflowBlockersDoc, AppError> {
    let path = blockers_path(project_path, mission_id);
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let mut doc: WorkflowBlockersDoc = serde_json::from_str(&content)?;
        if let Ok(state) = read_state(project_path, mission_id) {
            let features = read_features(project_path, mission_id)
                .map(|doc| doc.features)
                .unwrap_or_default();
            hydrate_pending_blocker_scope_from_state(&state, &mut doc);
            synthesize_runtime_blockers(mission_id, &state, &features, &mut doc);
            expand_blocker_related_task_scope(&features, &mut doc);
        }
        return Ok(doc);
    }

    let mut blockers_doc = derive_blockers(
        mission_id,
        read_pending_review_decision(project_path, mission_id)?.as_ref(),
        read_pending_knowledge_decision(project_path, mission_id)?.as_ref(),
    );
    if let Ok(state) = read_state(project_path, mission_id) {
        let features = read_features(project_path, mission_id)
            .map(|doc| doc.features)
            .unwrap_or_default();
        hydrate_pending_blocker_scope_from_state(&state, &mut blockers_doc);
        synthesize_runtime_blockers(mission_id, &state, &features, &mut blockers_doc);
        expand_blocker_related_task_scope(&features, &mut blockers_doc);
    }

    Ok(blockers_doc)
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

pub fn read_layer1_active_foreshadowing(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<serde_json::Value>, AppError> {
    read_optional_json(&layer1_active_foreshadowing_path(project_path, mission_id))
}

pub fn read_layer1_snapshot(
    project_path: &Path,
    mission_id: &str,
) -> Result<Layer1Snapshot, AppError> {
    Ok(Layer1Snapshot {
        chapter_card: read_layer1_chapter_card(project_path, mission_id)?,
        recent_facts: read_layer1_recent_facts(project_path, mission_id)?,
        active_cast: read_layer1_active_cast(project_path, mission_id)?,
        active_foreshadowing: read_optional_json(&layer1_active_foreshadowing_path(
            project_path,
            mission_id,
        ))?,
        previous_summary: read_optional_json(&layer1_previous_summary_path(
            project_path,
            mission_id,
        ))?,
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

pub fn read_knowledge_bundles(
    project_path: &Path,
    mission_id: &str,
) -> Result<Vec<KnowledgeProposalBundle>, AppError> {
    let path = knowledge_bundles_path(project_path, mission_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let entries: Vec<KnowledgeProposalBundle> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            serde_json::from_str(l)
                .map_err(|e| {
                    tracing::warn!(target: "mission", line = %l, "knowledge bundle parse error: {e}")
                })
                .ok()
        })
        .collect();
    Ok(entries)
}

pub fn read_knowledge_deltas(
    project_path: &Path,
    mission_id: &str,
) -> Result<Vec<KnowledgeDelta>, AppError> {
    let path = knowledge_deltas_path(project_path, mission_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let entries: Vec<KnowledgeDelta> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            serde_json::from_str(l)
                .map_err(|e| {
                    tracing::warn!(target: "mission", line = %l, "knowledge delta parse error: {e}")
                })
                .ok()
        })
        .collect();
    Ok(entries)
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
    atomic_write_json(&features_path(project_path, mission_id), doc)?;
    let _ = refresh_job_snapshot(project_path, mission_id);
    Ok(())
}

pub fn write_state(project_path: &Path, mission_id: &str, doc: &StateDoc) -> Result<(), AppError> {
    atomic_write_json(&state_path(project_path, mission_id), doc)?;
    sync_workflow_status_for_state(project_path, mission_id, &doc.state, doc.updated_at)?;
    let _ = refresh_job_snapshot(project_path, mission_id);
    Ok(())
}

pub fn write_workflow(
    project_path: &Path,
    mission_id: &str,
    doc: &WorkflowDoc,
) -> Result<(), AppError> {
    atomic_write_json(&workflow_path(project_path, mission_id), doc)?;
    let _ = refresh_job_snapshot(project_path, mission_id);
    Ok(())
}

pub fn write_workflow_blockers(
    project_path: &Path,
    mission_id: &str,
    doc: &WorkflowBlockersDoc,
) -> Result<(), AppError> {
    atomic_write_json(&blockers_path(project_path, mission_id), doc)?;
    let _ = refresh_job_snapshot(project_path, mission_id);
    Ok(())
}

pub fn write_job_snapshot(
    project_path: &Path,
    mission_id: &str,
    doc: &JobSnapshot,
) -> Result<(), AppError> {
    atomic_write_json(&job_snapshot_path(project_path, mission_id), doc)
}

pub fn refresh_job_snapshot(
    project_path: &Path,
    mission_id: &str,
) -> Result<JobSnapshot, AppError> {
    let snapshot = build_job_snapshot(project_path, mission_id)?;
    write_job_snapshot(project_path, mission_id, &snapshot)?;
    Ok(snapshot)
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

pub fn append_task_result(
    project_path: &Path,
    mission_id: &str,
    entry: &AgentTaskResult,
) -> Result<(), AppError> {
    let path = task_results_path(project_path, mission_id);
    let line = serde_json::to_string(entry)? + "\n";
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    file.flush()?;
    let _ = refresh_job_snapshot(project_path, mission_id);
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
    atomic_write_json(&path, doc)?;
    let _ = refresh_workflow_blockers(project_path, mission_id);
    Ok(())
}

pub fn clear_pending_review_decision(
    project_path: &Path,
    mission_id: &str,
) -> Result<(), AppError> {
    let path = pending_review_decision_path(project_path, mission_id);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let _ = refresh_workflow_blockers(project_path, mission_id);
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
    atomic_write_json(&path, doc)?;
    let _ = refresh_workflow_blockers(project_path, mission_id);
    Ok(())
}

pub fn clear_pending_knowledge_decision(
    project_path: &Path,
    mission_id: &str,
) -> Result<(), AppError> {
    let path = pending_knowledge_decision_path(project_path, mission_id);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let _ = refresh_workflow_blockers(project_path, mission_id);
    Ok(())
}

fn blocker_state(blockers: &[WorkflowBlocker]) -> Option<MissionState> {
    let blockers = blockers
        .iter()
        .filter(|blocker| blocker.blocking)
        .collect::<Vec<_>>();
    if blockers.is_empty() {
        return None;
    }

    if blockers
        .iter()
        .any(|blocker| blocker.kind == super::blockers::WorkflowBlockerKind::ReviewGate)
    {
        return Some(MissionState::WaitingReview);
    }
    if blockers
        .iter()
        .any(|blocker| blocker.kind == super::blockers::WorkflowBlockerKind::KnowledgeDecision)
    {
        return Some(MissionState::WaitingKnowledgeDecision);
    }
    if blockers
        .iter()
        .any(|blocker| blocker.kind == super::blockers::WorkflowBlockerKind::UserClarification)
    {
        return Some(MissionState::WaitingUser);
    }

    Some(MissionState::Blocked)
}

fn related_task_ids(seed_feature_id: &str, features: &[Feature]) -> Vec<String> {
    let mut adjacency = std::collections::HashMap::<&str, Vec<&str>>::new();
    for feature in features {
        adjacency.entry(feature.id.as_str()).or_default();
    }
    for feature in features {
        for dep in &feature.depends_on {
            let dep = dep.as_str();
            if adjacency.contains_key(dep) {
                adjacency.entry(feature.id.as_str()).or_default().push(dep);
                adjacency.entry(dep).or_default().push(feature.id.as_str());
            }
        }
    }

    if !adjacency.contains_key(seed_feature_id) {
        return Vec::new();
    }

    let mut queue = std::collections::VecDeque::from([seed_feature_id.to_string()]);
    let mut visited = std::collections::BTreeSet::new();
    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if let Some(neighbors) = adjacency.get(current.as_str()) {
            for neighbor in neighbors {
                if !visited.contains(*neighbor) {
                    queue.push_back((*neighbor).to_string());
                }
            }
        }
    }

    visited.into_iter().collect()
}

fn normalize_task_ids(task_ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut normalized = std::collections::BTreeSet::new();
    for task_id in task_ids {
        let trimmed = task_id.trim();
        if !trimmed.is_empty() {
            normalized.insert(trimmed.to_string());
        }
    }
    normalized.into_iter().collect()
}

fn expand_blocker_related_task_scope(features: &[Feature], blockers_doc: &mut WorkflowBlockersDoc) {
    for blocker in &mut blockers_doc.blockers {
        if !blocker.blocking {
            continue;
        }

        blocker.related_task_ids =
            normalize_task_ids(std::mem::take(&mut blocker.related_task_ids));

        if let Some(feature_id) = blocker
            .feature_id
            .as_deref()
            .map(str::trim)
            .filter(|feature_id| !feature_id.is_empty())
            .map(str::to_string)
        {
            blocker.feature_id = Some(feature_id.clone());
            let related = related_task_ids(&feature_id, features);
            if !related.is_empty() {
                blocker.related_task_ids = normalize_task_ids(
                    blocker
                        .related_task_ids
                        .iter()
                        .cloned()
                        .chain(related.into_iter()),
                );
            }

            let wave_id_missing = blocker
                .wave_id
                .as_ref()
                .map(|wave_id| wave_id.trim().is_empty())
                .unwrap_or(true);
            if wave_id_missing {
                blocker.wave_id = Some(format!("feature:{feature_id}"));
            }
        }
    }
}

fn hydrate_pending_blocker_scope_from_state(
    state: &StateDoc,
    blockers_doc: &mut WorkflowBlockersDoc,
) {
    let Some(feature_id) = runtime_blocker_feature_id(state) else {
        return;
    };

    for blocker in &mut blockers_doc.blockers {
        let missing_feature_id = blocker
            .feature_id
            .as_deref()
            .map(str::trim)
            .map(|value| value.is_empty())
            .unwrap_or(true);

        if !missing_feature_id {
            continue;
        }

        if matches!(
            blocker.kind,
            WorkflowBlockerKind::ReviewGate | WorkflowBlockerKind::KnowledgeDecision
        ) {
            blocker.feature_id = Some(feature_id.clone());
        }
    }
}

fn synthesize_runtime_blockers(
    mission_id: &str,
    state: &StateDoc,
    features: &[Feature],
    blockers_doc: &mut WorkflowBlockersDoc,
) {
    if !blockers_doc.blockers.is_empty() {
        return;
    }

    let feature_id = runtime_blocker_feature_id(state);
    let synthetic = match state.state {
        MissionState::WaitingUser => Some(WorkflowBlocker::user_clarification(
            mission_id,
            "mission paused pending user clarification",
            feature_id.clone(),
        )),
        MissionState::Blocked => Some(WorkflowBlocker::external_dependency(
            mission_id,
            "mission blocked pending an external dependency or manual intervention",
            feature_id.clone(),
        )),
        _ => None,
    };

    if let Some(blocker) = synthetic {
        let blocker = blocker.with_timestamps(state.updated_at, state.updated_at);
        let expanded = if let Some(feature_id) = blocker.feature_id.clone() {
            blocker
                .with_related_task_ids(related_task_ids(&feature_id, features))
                .with_wave_id(Some(format!("feature:{feature_id}")))
        } else {
            blocker
        };
        blockers_doc.blockers.push(expanded);
        blockers_doc.updated_at = chrono::Utc::now().timestamp_millis();
    }
}

fn sync_workflow_status_for_state(
    project_path: &Path,
    mission_id: &str,
    state: &MissionState,
    updated_at: i64,
) -> Result<(), AppError> {
    let mut workflow = read_workflow(project_path, mission_id)?;
    let new_status = WorkflowStatus::from_mission_state(state);
    if workflow.status != new_status || workflow.updated_at != updated_at {
        workflow.status = new_status;
        workflow.updated_at = updated_at;
        write_workflow(project_path, mission_id, &workflow)?;
    }
    Ok(())
}

pub fn refresh_workflow_blockers(
    project_path: &Path,
    mission_id: &str,
) -> Result<WorkflowBlockersDoc, AppError> {
    let mut state = match read_state(project_path, mission_id) {
        Ok(state) => state,
        Err(err) => return Err(err),
    };
    let pending_review = read_pending_review_decision(project_path, mission_id)?;
    let pending_knowledge = read_pending_knowledge_decision(project_path, mission_id)?;
    let features_doc = read_features(project_path, mission_id)?;
    let mut blockers_doc = derive_blockers(
        mission_id,
        pending_review.as_ref(),
        pending_knowledge.as_ref(),
    );
    hydrate_pending_blocker_scope_from_state(&state, &mut blockers_doc);
    synthesize_runtime_blockers(
        mission_id,
        &state,
        &features_doc.features,
        &mut blockers_doc,
    );
    expand_blocker_related_task_scope(&features_doc.features, &mut blockers_doc);

    write_workflow_blockers(project_path, mission_id, &blockers_doc)?;

    if matches!(
        state.state,
        MissionState::Completed | MissionState::Cancelled | MissionState::Failed
    ) {
        return Ok(blockers_doc);
    }

    let desired_state = blocker_state(&blockers_doc.blockers).unwrap_or_else(|| {
        if matches!(
            state.state,
            MissionState::WaitingReview
                | MissionState::WaitingKnowledgeDecision
                | MissionState::WaitingUser
                | MissionState::Blocked
        ) {
            MissionState::Paused
        } else {
            state.state.clone()
        }
    });

    if desired_state != state.state {
        state.state = desired_state;
        state.updated_at = chrono::Utc::now().timestamp_millis();
        write_state(project_path, mission_id, &state)?;
    }

    Ok(blockers_doc)
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

// ── Macro workflow artifact I/O (M5) ────────────────────────────

use super::macro_types::{MacroWorkflowConfig, MacroWorkflowState};

pub const MACRO_DIR: &str = "macro";

pub fn macro_dir(project_path: &Path, mission_id: &str) -> PathBuf {
    mission_dir(project_path, mission_id).join(MACRO_DIR)
}

pub fn macro_config_path(project_path: &Path, mission_id: &str) -> PathBuf {
    macro_dir(project_path, mission_id).join("config.json")
}

pub fn macro_state_path(project_path: &Path, mission_id: &str) -> PathBuf {
    macro_dir(project_path, mission_id).join("state.json")
}

pub fn macro_checkpoints_path(project_path: &Path, mission_id: &str) -> PathBuf {
    macro_dir(project_path, mission_id).join("checkpoints.jsonl")
}

pub fn read_macro_config(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<MacroWorkflowConfig>, AppError> {
    let path = macro_config_path(project_path, mission_id);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let cfg: MacroWorkflowConfig = serde_json::from_str(&content)?;
    Ok(Some(cfg))
}

pub fn read_macro_state(
    project_path: &Path,
    mission_id: &str,
) -> Result<Option<MacroWorkflowState>, AppError> {
    let path = macro_state_path(project_path, mission_id);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let st: MacroWorkflowState = serde_json::from_str(&content)?;
    Ok(Some(st))
}

pub fn write_macro_config(
    project_path: &Path,
    mission_id: &str,
    config: &MacroWorkflowConfig,
) -> Result<(), AppError> {
    let dir = macro_dir(project_path, mission_id);
    std::fs::create_dir_all(&dir)?;
    atomic_write_json(&macro_config_path(project_path, mission_id), config)
}

pub fn write_macro_state(
    project_path: &Path,
    mission_id: &str,
    state: &MacroWorkflowState,
) -> Result<(), AppError> {
    let dir = macro_dir(project_path, mission_id);
    std::fs::create_dir_all(&dir)?;
    atomic_write_json(&macro_state_path(project_path, mission_id), state)
}

pub fn append_macro_checkpoint(
    project_path: &Path,
    mission_id: &str,
    entry: &serde_json::Value,
) -> Result<(), AppError> {
    let dir = macro_dir(project_path, mission_id);
    std::fs::create_dir_all(&dir)?;
    let path = macro_checkpoints_path(project_path, mission_id);
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::contextpack_types::{TokenBudget, CONTEXTPACK_SCHEMA_VERSION};
    use crate::mission::layer1_types::{
        ChapterCardStatus, ChapterWorkflowKind, LAYER1_SCHEMA_VERSION,
    };
    use std::fs;

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

    fn sample_features_with_dependency_chain() -> Vec<Feature> {
        vec![
            Feature {
                id: "f1".to_string(),
                status: FeatureStatus::Pending,
                description: "Seed feature".to_string(),
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
                description: "Depends on f1".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: vec!["f1".to_string()],
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: Vec::new(),
            },
            Feature {
                id: "f3".to_string(),
                status: FeatureStatus::Pending,
                description: "Independent feature".to_string(),
                skill: String::new(),
                preconditions: Vec::new(),
                depends_on: Vec::new(),
                expected_behavior: Vec::new(),
                verification_steps: Vec::new(),
                write_paths: Vec::new(),
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
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Draft,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());

        let dir = init_mission_dir(
            &project,
            mission_id,
            "# My Mission\nGoal: test",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();
        assert!(dir.exists());

        // Verify files exist
        assert!(mission_md_path(&project, mission_id).exists());
        assert!(features_path(&project, mission_id).exists());
        assert!(state_path(&project, mission_id).exists());
        assert!(task_results_path(&project, mission_id).exists());
        assert!(handoffs_path(&project, mission_id).exists());
        assert!(worker_runs_path(&project, mission_id).exists());
        assert!(job_snapshot_path(&project, mission_id).exists());

        // Read back
        let md = read_mission_md(&project, mission_id).unwrap();
        assert!(md.contains("My Mission"));

        let features = read_features(&project, mission_id).unwrap();
        assert_eq!(features.features.len(), 2);
        assert_eq!(features.features[0].id, "f1");

        let state = read_state(&project, mission_id).unwrap();
        assert_eq!(state.state, MissionState::AwaitingInput);
        let workflow = read_workflow(&project, mission_id).unwrap();
        assert_eq!(workflow.status, WorkflowStatus::Draft);
        let blockers = read_workflow_blockers(&project, mission_id).unwrap();
        assert!(blockers.blockers.is_empty());

        let task_results = read_task_results(&project, mission_id).unwrap();
        assert!(task_results.is_empty());
        let snapshot = read_job_snapshot(&project, mission_id).unwrap();
        assert_eq!(snapshot.job_id, mission_id);
        assert!(snapshot.task_results.is_empty());

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
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Draft,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "test",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();

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
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Draft,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "t",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();

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
            rules_fingerprint: None,
            rules_sources: vec![],
            bound_validation_profile_id: None,
            bound_style_template_id: None,
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
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Draft,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "t",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();

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
    fn test_append_and_read_task_results() {
        let project = temp_project_dir();
        let mission_id = "mis_test_task_results";

        let features_doc = FeaturesDoc::new(mission_id.to_string(), "T".to_string(), Vec::new());
        let state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Draft,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "t",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();

        let result = AgentTaskResult {
            task_id: "f1".to_string(),
            actor_id: "wk_1".to_string(),
            goal: "Write chapter".to_string(),
            result_summary: "draft updated".to_string(),
            ..AgentTaskResult::default()
        };

        append_task_result(&project, mission_id, &result).unwrap();

        let results = read_task_results(&project, mission_id).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_id, "f1");
        assert_eq!(results[0].normalized_summary(), "draft updated");
        let snapshot = read_job_snapshot(&project, mission_id).unwrap();
        assert_eq!(snapshot.task_results.len(), 1);
        assert_eq!(snapshot.task_results[0].parent_task_id, "f1");

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_read_job_snapshot_rebuilds_when_task_results_append_skips_refresh() {
        let project = temp_project_dir();
        let mission_id = "mis_test_snapshot_rebuild";

        let features_doc = FeaturesDoc::new(mission_id.to_string(), "T".to_string(), Vec::new());
        let state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Draft,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "t",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();

        // Simulate an orchestration path appending task results directly.
        let result = AgentTaskResult {
            task_id: "f1".to_string(),
            actor_id: "wk_1".to_string(),
            goal: "Write chapter".to_string(),
            result_summary: "draft updated".to_string(),
            ..AgentTaskResult::default()
        };
        let line = format!("{}\n", serde_json::to_string(&result).unwrap());
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(task_results_path(&project, mission_id))
            .unwrap();
        file.write_all(line.as_bytes()).unwrap();
        file.flush().unwrap();

        let snapshot = read_job_snapshot(&project, mission_id).unwrap();
        assert_eq!(snapshot.task_results.len(), 1);
        assert_eq!(snapshot.task_results[0].parent_task_id, "f1");

        let persisted = read_job_snapshot(&project, mission_id).unwrap();
        assert_eq!(persisted.task_results.len(), 1);

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_expand_blocker_related_task_scope_merges_existing_ids_and_sets_wave() {
        let features = sample_features_with_dependency_chain();
        let mut blocker = WorkflowBlocker::external_dependency(
            "mis_test_scope",
            "blocked by dependency",
            Some("f1".to_string()),
        )
        .with_related_task_ids(vec!["custom_task".to_string()]);
        blocker.wave_id = Some("   ".to_string());

        let mut blockers_doc = WorkflowBlockersDoc {
            schema_version: crate::mission::blockers::WORKFLOW_BLOCKERS_SCHEMA_VERSION,
            mission_id: "mis_test_scope".to_string(),
            blockers: vec![blocker],
            updated_at: 0,
        };

        expand_blocker_related_task_scope(&features, &mut blockers_doc);

        assert_eq!(
            blockers_doc.blockers[0].related_task_ids,
            vec![
                "custom_task".to_string(),
                "f1".to_string(),
                "f2".to_string(),
            ]
        );
        assert_eq!(
            blockers_doc.blockers[0].wave_id.as_deref(),
            Some("feature:f1")
        );
    }

    #[test]
    fn test_read_job_snapshot_enriches_pending_review_blocker_scope_from_state() {
        let project = temp_project_dir();
        let mission_id = "mis_test_waiting_review_snapshot";

        let features_doc = FeaturesDoc::new(
            mission_id.to_string(),
            "T".to_string(),
            sample_features_with_dependency_chain(),
        );
        let mut state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        state_doc.current_feature_id = Some("  f1  ".to_string());
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Paused,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "t",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();
        write_pending_review_decision(
            &project,
            mission_id,
            &ReviewDecisionRequest {
                schema_version: 1,
                review_id: "rev_scope".to_string(),
                feature_id: None,
                scope_ref: "chapter:1".to_string(),
                target_refs: None,
                question: "review?".to_string(),
                options: vec!["accept".to_string()],
                context_summary: Vec::new(),
                created_at: 10,
            },
        )
        .unwrap();

        let snapshot = read_job_snapshot(&project, mission_id).unwrap();

        assert_eq!(
            snapshot.status,
            crate::mission::job_types::JobStatus::WaitingReview
        );
        assert_eq!(snapshot.blockers.len(), 1);
        assert_eq!(
            snapshot.blockers[0].kind,
            crate::mission::blockers::WorkflowBlockerKind::ReviewGate
        );
        assert_eq!(snapshot.blockers[0].feature_id.as_deref(), Some("f1"));
        assert_eq!(
            snapshot.blockers[0].related_task_ids,
            vec!["f1".to_string(), "f2".to_string()]
        );
        assert_eq!(snapshot.blockers[0].wave_id.as_deref(), Some("feature:f1"));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_read_workflow_blockers_enriches_pending_knowledge_scope_from_assignment() {
        let project = temp_project_dir();
        let mission_id = "mis_test_waiting_knowledge_blocker";

        let features_doc = FeaturesDoc::new(
            mission_id.to_string(),
            "T".to_string(),
            sample_features_with_dependency_chain(),
        );
        let mut state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        state_doc.current_worker_id = Some("wk_1".to_string());
        state_doc.assignments.insert(
            "wk_1".to_string(),
            WorkerAssignment {
                feature_id: "  f2  ".to_string(),
                attempt: 0,
                started_at: 10,
                last_heartbeat_at: 20,
            },
        );
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Paused,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "t",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();
        write_pending_knowledge_decision(
            &project,
            mission_id,
            &PendingKnowledgeDecision {
                schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
                bundle_id: "bundle_1".to_string(),
                delta_id: "delta_1".to_string(),
                scope_ref: "chapter:1".to_string(),
                conflicts: Vec::new(),
                created_at: 10,
            },
        )
        .unwrap();

        let blockers = read_workflow_blockers(&project, mission_id).unwrap();

        assert_eq!(blockers.blockers.len(), 1);
        assert_eq!(
            blockers.blockers[0].kind,
            crate::mission::blockers::WorkflowBlockerKind::KnowledgeDecision
        );
        assert_eq!(blockers.blockers[0].feature_id.as_deref(), Some("f2"));
        assert_eq!(
            blockers.blockers[0].related_task_ids,
            vec!["f1".to_string(), "f2".to_string()]
        );
        assert_eq!(blockers.blockers[0].wave_id.as_deref(), Some("feature:f2"));

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn test_job_snapshot_keeps_latest_task_result_per_task() {
        let project = temp_project_dir();
        let mission_id = "mis_test_snapshot_latest_results";

        let features_doc = FeaturesDoc::new(mission_id.to_string(), "T".to_string(), Vec::new());
        let state_doc = StateDoc::new(
            mission_id.to_string(),
            project.to_string_lossy().to_string(),
        );
        let workflow_doc = WorkflowDoc::new(
            mission_id.to_string(),
            MissionWorkflowKind::AdHoc,
            WorkflowCreationReason::ExplicitMissionRequest,
            SummaryJobPolicy::ParentSessionSummary,
            WorkflowStatus::Running,
        );
        let blockers_doc = WorkflowBlockersDoc::empty(mission_id.to_string());
        init_mission_dir(
            &project,
            mission_id,
            "t",
            &features_doc,
            &state_doc,
            &workflow_doc,
            &blockers_doc,
        )
        .unwrap();

        append_task_result(
            &project,
            mission_id,
            &AgentTaskResult {
                task_id: "f1".to_string(),
                actor_id: "wk_1".to_string(),
                goal: "Write chapter".to_string(),
                status: crate::mission::result_types::TaskResultStatus::Failed,
                result_summary: "first attempt failed".to_string(),
                ..AgentTaskResult::default()
            },
        )
        .unwrap();
        append_task_result(
            &project,
            mission_id,
            &AgentTaskResult {
                task_id: "f1".to_string(),
                actor_id: "wk_2".to_string(),
                goal: "Write chapter".to_string(),
                status: crate::mission::result_types::TaskResultStatus::Completed,
                result_summary: "second attempt succeeded".to_string(),
                ..AgentTaskResult::default()
            },
        )
        .unwrap();

        let raw_results = read_task_results(&project, mission_id).unwrap();
        assert_eq!(raw_results.len(), 2);

        let snapshot = read_job_snapshot(&project, mission_id).unwrap();
        assert_eq!(snapshot.task_results.len(), 1);
        assert_eq!(snapshot.task_results[0].parent_task_id, "f1");
        assert_eq!(
            snapshot.task_results[0].status,
            crate::mission::result_types::TaskResultStatus::Completed
        );
        assert_eq!(
            snapshot.task_results[0].result_summary,
            "second attempt succeeded"
        );
        assert_eq!(snapshot.task_results[0].actor_id.as_deref(), Some("wk_2"));

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
