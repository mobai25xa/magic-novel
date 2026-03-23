use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use tokio::sync::Mutex as TokioMutex;

use crate::mission::agent_profile::AgentProfile;
use crate::mission::agent_profile::SessionSource;
use crate::mission::artifacts;
use crate::mission::blockers::WorkflowBlocker;
use crate::mission::events::MissionEventEmitter;
use crate::mission::job_types::{ResourceLock, ResourceLockKind, ResourceLockMode};
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::{ProcessManager, WorkerProcess};
use crate::mission::result_types::TaskResultStatus;
use crate::mission::types::INTEGRATOR_FEATURE_ID;
use crate::mission::types::*;
use crate::mission::worker_profile::{
    agent_profile_from_definition, builtin_general_worker_profile,
    builtin_integrator_worker_profile, AgentProfileSummary, WorkerRunEntry,
};
use crate::mission::worker_protocol::StartFeaturePayload;
use crate::models::AppError;
use crate::services::global_config;

use super::super::runtime::*;
use super::super::DelegateTransportMode;
use super::{MissionRunConfig, MissionStartConfig};

pub(super) fn enrich_integrator_feature_context(
    project_path: &std::path::Path,
    mission_id: &str,
    feature: &mut Feature,
) -> Result<(), AppError> {
    if feature.id != INTEGRATOR_FEATURE_ID {
        return Ok(());
    }

    let features_doc = artifacts::read_features(project_path, mission_id)?;
    let task_results = artifacts::read_task_results(project_path, mission_id)?;
    let handoffs = artifacts::read_handoffs(project_path, mission_id)?;

    let completed = features_doc
        .features
        .iter()
        .filter(|f| f.id != INTEGRATOR_FEATURE_ID && f.status == FeatureStatus::Completed)
        .count();
    let failed = features_doc
        .features
        .iter()
        .filter(|f| f.id != INTEGRATOR_FEATURE_ID && f.status == FeatureStatus::Failed)
        .count();

    let recent_results = if task_results.is_empty() {
        handoffs
            .iter()
            .rev()
            .take(20)
            .map(|h| {
                format!(
                    "- {} [{}]: {}",
                    h.feature_id,
                    if h.ok { "ok" } else { "failed" },
                    h.summary
                )
            })
            .collect::<Vec<_>>()
    } else {
        task_results
            .iter()
            .rev()
            .take(20)
            .map(|result| {
                let status = match result.status {
                    TaskResultStatus::Completed => "completed",
                    TaskResultStatus::Failed => "failed",
                    TaskResultStatus::Cancelled => "cancelled",
                    TaskResultStatus::Blocked => "blocked",
                };
                format!(
                    "- {} [{}]: {}",
                    result.task_id,
                    status,
                    result.normalized_summary()
                )
            })
            .collect::<Vec<_>>()
    };

    feature.preconditions = vec![
        format!("Mission id: {mission_id}"),
        format!("Completed features: {completed}"),
        format!("Failed features: {failed}"),
    ];
    feature.expected_behavior = vec![
        "Produce mission-level final summary".to_string(),
        "Highlight feature-level unresolved issues".to_string(),
        "Point out possible write conflict symptoms if observed".to_string(),
    ];
    feature.verification_steps = vec![
        "Read mission artifacts and compile final mission summary".to_string(),
        "Ensure unresolved items are explicit and actionable".to_string(),
        if recent_results.is_empty() {
            "No task results available yet".to_string()
        } else {
            format!("Recent task results:\n{}", recent_results.join("\n"))
        },
    ];

    Ok(())
}

pub(super) fn select_worker_profile_for_feature(
    feature: &Feature,
    worker_defs: &[global_config::WorkerDefinition],
) -> AgentProfile {
    let skill = feature.skill.trim();
    if !skill.is_empty() {
        if let Some(def) = worker_defs
            .iter()
            .find(|d| worker_definition_matches_skill(d.name.trim(), skill))
        {
            return agent_profile_from_definition(def);
        }

        if skill.eq_ignore_ascii_case("integrator") || feature.id == INTEGRATOR_FEATURE_ID {
            return builtin_integrator_worker_profile();
        }
    }

    if feature.id == INTEGRATOR_FEATURE_ID {
        // Prefer user-defined integrator worker if present.
        if let Some(def) = worker_defs
            .iter()
            .find(|d| d.name.trim().eq_ignore_ascii_case("integrator"))
        {
            return agent_profile_from_definition(def);
        }
        return builtin_integrator_worker_profile();
    }

    // Prefer explicit general-purpose worker names when present.
    for name in ["general-worker", "general"] {
        if let Some(def) = worker_defs
            .iter()
            .find(|d| d.name.trim().eq_ignore_ascii_case(name))
        {
            return agent_profile_from_definition(def);
        }
    }

    builtin_general_worker_profile()
}

fn worker_definition_matches_skill(definition_name: &str, skill: &str) -> bool {
    if skill.trim().is_empty() {
        return false;
    }

    let definition_name = definition_name.trim();
    let skill = skill.trim();

    definition_name.eq_ignore_ascii_case(skill)
        || definition_name.eq_ignore_ascii_case(&format!("{skill}-worker"))
        || skill
            .strip_suffix("-worker")
            .map(|alias| definition_name.eq_ignore_ascii_case(alias))
            .unwrap_or(false)
}

pub(super) fn update_worker_assignment_heartbeat(
    project_path: &std::path::Path,
    mission_id: &str,
    worker_id: &str,
) {
    if let Ok(mut state_doc) = artifacts::read_state(project_path, mission_id) {
        let now = chrono::Utc::now().timestamp_millis();
        if let Some(assignment) = state_doc.assignments.get_mut(worker_id) {
            assignment.last_heartbeat_at = now;
            state_doc.updated_at = now;
            let _ = artifacts::write_state(project_path, mission_id, &state_doc);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchedulingLock {
    key: String,
    mode: ResourceLockMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SchedulabilityExclusion {
    BlockingBlocker,
    ResourceLockConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FeatureSchedulability {
    Schedulable { reserved_locks: Vec<SchedulingLock> },
    ExcludedNow(SchedulabilityExclusion),
}

pub(super) fn delegate_transport_mode(start_config: &MissionStartConfig) -> DelegateTransportMode {
    start_config.delegate_transport
}

fn normalize_lock_scope(raw: &str) -> Option<String> {
    let mut scope = raw.trim().replace('\\', "/");
    while scope.contains("//") {
        scope = scope.replace("//", "/");
    }
    while scope.starts_with("./") {
        scope = scope[2..].to_string();
    }

    if scope.is_empty() {
        None
    } else {
        Some(scope)
    }
}

fn lock_kind_from_label(label: &str) -> Option<ResourceLockKind> {
    match label.trim().to_ascii_lowercase().as_str() {
        "file" => Some(ResourceLockKind::File),
        "chapter" => Some(ResourceLockKind::Chapter),
        "canon" => Some(ResourceLockKind::Canon),
        "review" => Some(ResourceLockKind::Review),
        "external" | "external_dependency" => Some(ResourceLockKind::ExternalDependency),
        _ => None,
    }
}

fn parse_resource_lock_spec(spec: &str) -> Option<ResourceLock> {
    let mut mode = ResourceLockMode::Exclusive;
    let mut raw = spec.trim();
    if raw.is_empty() {
        return None;
    }

    if let Some(next) = raw.strip_prefix("shared:") {
        mode = ResourceLockMode::Shared;
        raw = next.trim();
    } else if let Some(next) = raw.strip_prefix("exclusive:") {
        raw = next.trim();
    }

    let (lock_kind, scope) = if let Some((kind_raw, scope_raw)) = raw.split_once(':') {
        if let Some(kind) = lock_kind_from_label(kind_raw) {
            (kind, scope_raw)
        } else {
            (ResourceLockKind::File, raw)
        }
    } else {
        (ResourceLockKind::File, raw)
    };

    let scope = normalize_lock_scope(scope)?;
    let lock_id = format!("{:?}:{}", lock_kind, scope).to_ascii_lowercase();

    Some(
        ResourceLock {
            lock_id,
            lock_kind,
            scope,
            mode,
        }
        .normalized(),
    )
}

fn parse_resource_locks_from_preconditions(feature: &Feature) -> Vec<ResourceLock> {
    let mut locks = Vec::new();
    let mut seen = HashSet::new();

    for raw_line in &feature.preconditions {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let line_lower = line.to_ascii_lowercase();
        let payload = if let Some(rest) = line_lower
            .strip_prefix("resource_lock:")
            .and_then(|_| line.strip_prefix("resource_lock:"))
        {
            Some(rest)
        } else if let Some(rest) = line_lower
            .strip_prefix("resource_lock=")
            .and_then(|_| line.strip_prefix("resource_lock="))
        {
            Some(rest)
        } else if let Some(rest) = line_lower
            .strip_prefix("lock:")
            .and_then(|_| line.strip_prefix("lock:"))
        {
            Some(rest)
        } else if let Some(rest) = line_lower
            .strip_prefix("lock=")
            .and_then(|_| line.strip_prefix("lock="))
        {
            Some(rest)
        } else if let Some(rest) = line_lower
            .strip_prefix("resource_locks:")
            .and_then(|_| line.strip_prefix("resource_locks:"))
        {
            Some(rest)
        } else if let Some(rest) = line_lower
            .strip_prefix("resource_locks=")
            .and_then(|_| line.strip_prefix("resource_locks="))
        {
            Some(rest)
        } else {
            None
        };

        let Some(payload) = payload else {
            continue;
        };

        let payload = payload.trim();
        if payload.is_empty() {
            continue;
        }

        if payload.starts_with('[') {
            if let Ok(values) = serde_json::from_str::<Vec<ResourceLock>>(payload) {
                for lock in values.into_iter().map(ResourceLock::normalized) {
                    let key = format!(
                        "{}:{}:{:?}",
                        lock.lock_id.trim().to_ascii_lowercase(),
                        lock.scope.trim().to_ascii_lowercase(),
                        lock.mode
                    );
                    if seen.insert(key) {
                        locks.push(lock);
                    }
                }
            }
            continue;
        }

        for part in payload.split([',', ';']) {
            if let Some(lock) = parse_resource_lock_spec(part) {
                let key = format!(
                    "{}:{}:{:?}",
                    lock.lock_id.trim().to_ascii_lowercase(),
                    lock.scope.trim().to_ascii_lowercase(),
                    lock.mode
                );
                if seen.insert(key) {
                    locks.push(lock);
                }
            }
        }
    }

    locks
}

pub(super) fn feature_resource_locks(feature: &Feature) -> Vec<ResourceLock> {
    let mut locks = parse_resource_locks_from_preconditions(feature);
    let mut seen = locks
        .iter()
        .map(|lock| {
            format!(
                "{}:{}:{:?}",
                lock.lock_id.trim().to_ascii_lowercase(),
                lock.scope.trim().to_ascii_lowercase(),
                lock.mode
            )
        })
        .collect::<HashSet<_>>();

    for write_path in &feature.write_paths {
        let Some(scope) = normalize_lock_scope(write_path) else {
            continue;
        };
        let lock = ResourceLock {
            lock_id: format!("file:{scope}"),
            lock_kind: ResourceLockKind::File,
            scope,
            mode: ResourceLockMode::Exclusive,
        }
        .normalized();
        let key = format!(
            "{}:{}:{:?}",
            lock.lock_id.trim().to_ascii_lowercase(),
            lock.scope.trim().to_ascii_lowercase(),
            lock.mode
        );
        if seen.insert(key) {
            locks.push(lock);
        }
    }

    locks
}

pub(super) fn feature_session_id(
    mission_id: &str,
    feature_id: &str,
    worker_id: &str,
    attempt: u32,
) -> String {
    format!("worker_{mission_id}_{feature_id}_{worker_id}_{attempt}")
}

fn scheduling_locks_for_feature(feature: &Feature) -> Vec<SchedulingLock> {
    feature_resource_locks(feature)
        .into_iter()
        .map(|lock| {
            let key = if lock.lock_id.trim().is_empty() {
                format!("{:?}:{}", lock.lock_kind, lock.scope).to_ascii_lowercase()
            } else {
                lock.lock_id.trim().to_ascii_lowercase()
            };
            SchedulingLock {
                key,
                mode: lock.mode,
            }
        })
        .collect()
}

fn scheduling_locks_conflict(left: &SchedulingLock, right: &SchedulingLock) -> bool {
    if left.key != right.key {
        return false;
    }

    !matches!(
        (left.mode, right.mode),
        (ResourceLockMode::Shared, ResourceLockMode::Shared)
    )
}

fn occupied_scheduling_locks(
    state_doc: &StateDoc,
    feature_by_id: &HashMap<&str, &Feature>,
) -> Vec<SchedulingLock> {
    let mut occupied_locks = Vec::new();

    for assignment in state_doc.assignments.values() {
        if let Some(feature) = feature_by_id.get(assignment.feature_id.as_str()) {
            occupied_locks.extend(scheduling_locks_for_feature(feature));
        }
    }

    occupied_locks
}

fn related_feature_ids(seed_feature_id: &str, features: &[Feature]) -> HashSet<String> {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
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

    let mut related = HashSet::new();
    if !adjacency.contains_key(seed_feature_id) {
        return related;
    }

    let mut queue = VecDeque::from([seed_feature_id.to_string()]);
    while let Some(current) = queue.pop_front() {
        if !related.insert(current.clone()) {
            continue;
        }

        if let Some(neighbors) = adjacency.get(current.as_str()) {
            for neighbor in neighbors {
                if !related.contains(*neighbor) {
                    queue.push_back((*neighbor).to_string());
                }
            }
        }
    }

    related
}

fn blocker_blocks_feature(
    blocker: &WorkflowBlocker,
    feature: &Feature,
    features: &[Feature],
    related_cache: &mut HashMap<String, HashSet<String>>,
) -> bool {
    if blocker.blocks_task(&feature.id) {
        return true;
    }

    if !blocker.blocking {
        return false;
    }

    let mut seeds = blocker.related_task_ids.clone();
    if let Some(feature_id) = blocker.feature_id.clone() {
        seeds.push(feature_id);
    }

    if seeds.is_empty() {
        return true;
    }

    for seed in seeds {
        let related = related_cache
            .entry(seed.clone())
            .or_insert_with(|| related_feature_ids(&seed, features));
        if related.contains(&feature.id) {
            return true;
        }
    }

    false
}

fn evaluate_feature_schedulability_now(
    feature: &Feature,
    features: &[Feature],
    blocking_blockers: &[WorkflowBlocker],
    occupied_locks: &[SchedulingLock],
    related_cache: &mut HashMap<String, HashSet<String>>,
) -> FeatureSchedulability {
    if !blocking_blockers.is_empty()
        && blocking_blockers
            .iter()
            .any(|blocker| blocker_blocks_feature(blocker, feature, features, related_cache))
    {
        return FeatureSchedulability::ExcludedNow(SchedulabilityExclusion::BlockingBlocker);
    }

    let candidate_locks = scheduling_locks_for_feature(feature);
    let conflicts_with_occupied = candidate_locks.iter().any(|lock| {
        occupied_locks
            .iter()
            .any(|existing| scheduling_locks_conflict(lock, existing))
    });
    if conflicts_with_occupied {
        return FeatureSchedulability::ExcludedNow(SchedulabilityExclusion::ResourceLockConflict);
    }

    FeatureSchedulability::Schedulable {
        reserved_locks: candidate_locks,
    }
}

fn select_schedulable_ready_features(
    ready_features: Vec<Feature>,
    features: &[Feature],
    blocking_blockers: &[WorkflowBlocker],
    mut occupied_locks: Vec<SchedulingLock>,
    available_slots: usize,
) -> Vec<Feature> {
    let mut related_cache: HashMap<String, HashSet<String>> = HashMap::new();
    let mut schedulable_features = Vec::new();

    for feature in ready_features {
        match evaluate_feature_schedulability_now(
            &feature,
            features,
            blocking_blockers,
            &occupied_locks,
            &mut related_cache,
        ) {
            FeatureSchedulability::Schedulable { reserved_locks } => {
                occupied_locks.extend(reserved_locks);
                schedulable_features.push(feature);

                if schedulable_features.len() >= available_slots {
                    break;
                }
            }
            FeatureSchedulability::ExcludedNow(_) => {}
        }
    }

    schedulable_features
}

pub(super) async fn spawn_and_initialize_worker(
    project_path: &std::path::Path,
    project_path_str: &str,
    mission_id: &str,
    requested_worker_id: Option<String>,
) -> Result<Arc<TokioMutex<WorkerProcess>>, AppError> {
    let worker_binary = ProcessManager::find_worker_binary()?;
    let mission_dir = artifacts::mission_dir(project_path, mission_id)
        .to_string_lossy()
        .to_string();
    let worker_id = requested_worker_id.unwrap_or_else(|| format!("wk_{}", uuid::Uuid::new_v4()));
    let pm = ProcessManager::new(worker_binary);
    let worker = pm.spawn(&worker_id)?;

    if let Ok(mut state_doc) = artifacts::read_state(project_path, mission_id) {
        state_doc
            .worker_pids
            .insert(worker_id.clone(), worker.pid());
        state_doc.updated_at = chrono::Utc::now().timestamp_millis();
        let _ = artifacts::write_state(project_path, mission_id, &state_doc);
    }

    if let Err(e) = worker.initialize(project_path_str, &mission_dir).await {
        clear_worker_from_state(project_path, mission_id, &worker_id);
        return Err(e);
    }

    Ok(Arc::new(TokioMutex::new(worker)))
}

pub(super) async fn schedule_ready_features(
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    mission_id: &str,
    project_path: &std::path::Path,
    project_path_str: &str,
    start_config: &MissionStartConfig,
    emit_orchestrator_transition: bool,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, AppError> {
    let blockers_doc = artifacts::refresh_workflow_blockers(project_path, mission_id)?;
    let state_doc = artifacts::read_state(project_path, mission_id)?;
    if !matches!(
        state_doc.state,
        MissionState::Running | MissionState::OrchestratorTurn
    ) {
        return Ok(Vec::new());
    }
    let active_workers = state_doc
        .assignments
        .len()
        .max(list_worker_handles(mission_id).len());
    if active_workers >= start_config.max_workers {
        return Ok(Vec::new());
    }

    let available_slots = start_config.max_workers.saturating_sub(active_workers);
    if available_slots == 0 {
        return Ok(Vec::new());
    }

    let features_doc = artifacts::read_features(project_path, mission_id)?;
    let candidate_limit = features_doc.features.len().max(available_slots);
    let ready_features = orch.ready_pending_features(candidate_limit)?;
    if ready_features.is_empty() {
        return Ok(Vec::new());
    }

    let blocking_blockers = blockers_doc
        .blockers
        .iter()
        .filter(|blocker| blocker.blocking)
        .cloned()
        .collect::<Vec<_>>();

    let feature_by_id = features_doc
        .features
        .iter()
        .map(|feature| (feature.id.as_str(), feature))
        .collect::<HashMap<_, _>>();

    let occupied_locks = occupied_scheduling_locks(&state_doc, &feature_by_id);
    let schedulable_features = select_schedulable_ready_features(
        ready_features,
        &features_doc.features,
        &blocking_blockers,
        occupied_locks,
        available_slots,
    );

    if schedulable_features.is_empty() {
        return Ok(Vec::new());
    }

    let worker_defs = global_config::load_worker_definitions();
    let transport_mode = delegate_transport_mode(start_config);

    let mut started = Vec::new();

    for (idx, feature) in schedulable_features.into_iter().enumerate() {
        let mut feature = feature;
        let _ = enrich_integrator_feature_context(project_path, mission_id, &mut feature);

        let worker_profile = select_worker_profile_for_feature(&feature, &worker_defs);
        let delegate_feature = feature.clone();
        let delegate_profile = worker_profile.clone();
        let worker_id = format!("wk_{}", uuid::Uuid::new_v4());
        let attempt = 0_u32;

        let start_result = match transport_mode {
            DelegateTransportMode::Process => {
                let worker_arc = spawn_and_initialize_worker(
                    project_path,
                    project_path_str,
                    mission_id,
                    Some(worker_id.clone()),
                )
                .await?;
                let delegate_worker = {
                    let worker = worker_arc.lock().await;
                    worker.clone()
                };

                let start_result = {
                    let worker = worker_arc.lock().await;
                    start_feature_on_worker(
                        orch,
                        &*worker,
                        emitter,
                        project_path,
                        mission_id,
                        feature.clone(),
                        &start_config.run_config,
                        &worker_id,
                        attempt,
                        worker_profile,
                        SessionSource::WorkflowJob,
                        start_config.parent_session_id.as_deref(),
                        start_config.parent_turn_id,
                        true,
                        emit_orchestrator_transition && idx == 0,
                    )
                    .await
                };

                match start_result {
                    Ok(feature_id) => {
                        insert_worker_handle(
                            mission_id,
                            worker_id.clone(),
                            MissionWorkerHandle {
                                worker: Arc::clone(&worker_arc),
                                attempt,
                            },
                        );

                        super::supervision::spawn_process_delegate_supervision_task(
                            app_handle.clone(),
                            mission_id.to_string(),
                            project_path_str.to_string(),
                            worker_id.clone(),
                            delegate_worker,
                            delegate_feature,
                            delegate_profile,
                            start_config.clone(),
                            attempt,
                        );
                        Ok(feature_id)
                    }
                    Err(error) => {
                        clear_worker_from_state(project_path, mission_id, &worker_id);
                        rollback_active_feature_to_pending(
                            orch,
                            project_path,
                            mission_id,
                            Some(&worker_id),
                        );
                        Err(error)
                    }
                }
            }
            DelegateTransportMode::InProcess => {
                let _ = emitter.progress_entry(&format!(
                    "starting {} via in-process delegate transport",
                    feature.id
                ));
                let start_result = start_feature_in_process(
                    orch,
                    emitter,
                    project_path,
                    mission_id,
                    feature.clone(),
                    &start_config.run_config,
                    &worker_id,
                    attempt,
                    worker_profile,
                    emit_orchestrator_transition && idx == 0,
                )
                .await;

                match start_result {
                    Ok(feature_id) => {
                        super::supervision::spawn_in_process_delegate_supervision_task(
                            app_handle.clone(),
                            mission_id.to_string(),
                            project_path_str.to_string(),
                            worker_id.clone(),
                            delegate_feature,
                            delegate_profile,
                            start_config.clone(),
                            attempt,
                        );
                        Ok(feature_id)
                    }
                    Err(error) => {
                        clear_worker_from_state(project_path, mission_id, &worker_id);
                        rollback_active_feature_to_pending(
                            orch,
                            project_path,
                            mission_id,
                            Some(&worker_id),
                        );
                        Err(error)
                    }
                }
            }
        };

        match start_result {
            Ok(feature_id) => {
                started.push(feature_id);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(started)
}

pub(super) async fn start_feature_on_worker(
    orch: &Orchestrator<'_>,
    worker: &WorkerProcess,
    emitter: &MissionEventEmitter,
    project_path: &std::path::Path,
    mission_id: &str,
    feature: Feature,
    run_config: &MissionRunConfig,
    worker_id: &str,
    attempt: u32,
    agent_profile: AgentProfile,
    session_source: SessionSource,
    parent_session_id: Option<&str>,
    parent_turn_id: Option<u32>,
    rollback_to_pending_on_start_error: bool,
    emit_orchestrator_transition: bool,
) -> Result<String, AppError> {
    let session_id = feature_session_id(mission_id, &feature.id, worker_id, attempt);

    let effective_model = agent_profile
        .model
        .as_deref()
        .unwrap_or(run_config.model.as_str())
        .trim()
        .to_string();
    let profile_summary = AgentProfileSummary::from_agent_profile(&agent_profile);

    let _ = artifacts::append_worker_run(
        project_path,
        mission_id,
        &WorkerRunEntry {
            schema_version: MISSION_SCHEMA_VERSION,
            ts: chrono::Utc::now().timestamp_millis(),
            mission_id: mission_id.to_string(),
            feature_id: feature.id.clone(),
            worker_id: worker_id.to_string(),
            attempt,
            profile: profile_summary,
            provider: run_config.provider.clone(),
            model: effective_model.clone(),
        },
    );

    orch.start_feature(&feature.id, worker_id, attempt)?;
    emitter.worker_started(worker_id, &feature.id)?;

    // M5: update macro state on feature start
    let _ = super::super::macro_commands::update_macro_state_on_feature_event(
        project_path,
        mission_id,
        &feature.id,
        &FeatureStatus::InProgress,
        Some(emitter),
    );

    if emit_orchestrator_transition {
        emitter.state_changed("orchestrator_turn", "running")?;
    }

    if let Err(e) = worker
        .start_feature_and_wait_ack(
            StartFeaturePayload {
                feature: feature.clone(),
                session_id,
                model: effective_model,
                provider: run_config.provider.clone(),
                base_url: run_config.base_url.clone(),
                api_key: run_config.api_key.clone(),
                mission_id: mission_id.to_string(),
                worker_id: worker_id.to_string(),
                agent_profile: Some(agent_profile),
                session_source,
                parent_session_id: parent_session_id.map(str::to_string),
                parent_turn_id,
            },
            std::time::Duration::from_secs(15),
        )
        .await
    {
        if rollback_to_pending_on_start_error {
            let _ = orch.update_feature_status(&feature.id, FeatureStatus::Pending);
        }
        return Err(e);
    }

    Ok(feature.id)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn start_feature_in_process(
    orch: &Orchestrator<'_>,
    emitter: &MissionEventEmitter,
    project_path: &std::path::Path,
    mission_id: &str,
    feature: Feature,
    run_config: &MissionRunConfig,
    worker_id: &str,
    attempt: u32,
    agent_profile: AgentProfile,
    emit_orchestrator_transition: bool,
) -> Result<String, AppError> {
    let effective_model = agent_profile
        .model
        .as_deref()
        .unwrap_or(run_config.model.as_str())
        .trim()
        .to_string();
    let profile_summary = AgentProfileSummary::from_agent_profile(&agent_profile);

    let _ = artifacts::append_worker_run(
        project_path,
        mission_id,
        &WorkerRunEntry {
            schema_version: MISSION_SCHEMA_VERSION,
            ts: chrono::Utc::now().timestamp_millis(),
            mission_id: mission_id.to_string(),
            feature_id: feature.id.clone(),
            worker_id: worker_id.to_string(),
            attempt,
            profile: profile_summary,
            provider: run_config.provider.clone(),
            model: effective_model,
        },
    );

    orch.start_feature(&feature.id, worker_id, attempt)?;
    emitter.worker_started(worker_id, &feature.id)?;

    let _ = super::super::macro_commands::update_macro_state_on_feature_event(
        project_path,
        mission_id,
        &feature.id,
        &FeatureStatus::InProgress,
        Some(emitter),
    );

    if emit_orchestrator_transition {
        emitter.state_changed("orchestrator_turn", "running")?;
    }

    Ok(feature.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(id: &str, description: &str, skill: &str) -> Feature {
        Feature {
            id: id.to_string(),
            status: FeatureStatus::Pending,
            description: description.to_string(),
            skill: skill.to_string(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }
    }

    fn worker_definition(
        name: &str,
        capability_preset: crate::mission::agent_profile::CapabilityPreset,
        forced_tools: &[&str],
    ) -> global_config::WorkerDefinition {
        global_config::WorkerDefinition {
            name: name.to_string(),
            display_name: name.to_string(),
            system_prompt: format!("system prompt for {name}"),
            mode: if matches!(
                capability_preset,
                crate::mission::agent_profile::CapabilityPreset::ReadOnlyReviewer
                    | crate::mission::agent_profile::CapabilityPreset::SummaryOnly
            ) {
                crate::agent_engine::types::AgentMode::Planning
            } else {
                crate::agent_engine::types::AgentMode::Writing
            },
            approval_mode: crate::agent_engine::types::ApprovalMode::Auto,
            clarification_mode: crate::agent_engine::types::ClarificationMode::HeadlessDefer,
            capability_preset,
            allow_delegate: false,
            allow_skill_activation: false,
            hidden_tools: Vec::new(),
            forced_tools: forced_tools.iter().map(|tool| tool.to_string()).collect(),
            max_rounds: Some(7),
            max_tool_calls: Some(21),
            model: None,
        }
    }

    #[test]
    fn select_worker_profile_prefers_exact_skill_match() {
        let feature = feature("feat-1", "Draft the opening scene", "plot-architect");
        let worker_defs = vec![
            worker_definition(
                "general-worker",
                crate::mission::agent_profile::CapabilityPreset::HeadlessWriter,
                &[],
            ),
            worker_definition(
                "plot-architect",
                crate::mission::agent_profile::CapabilityPreset::HeadlessWriter,
                &[],
            ),
        ];

        let profile = select_worker_profile_for_feature(&feature, &worker_defs);

        assert_eq!(profile.name, "plot-architect");
        assert_eq!(
            profile.capability_preset,
            crate::mission::agent_profile::CapabilityPreset::HeadlessWriter
        );
        assert_eq!(profile.max_rounds, 7);
        assert_eq!(profile.max_tool_calls, 21);
    }

    #[test]
    fn select_worker_profile_matches_conventional_worker_suffix_for_skill() {
        let feature = feature(
            "feat-2",
            "Need a continuity audit for chapter 3",
            "continuity",
        );
        let worker_defs = vec![
            worker_definition(
                "draft-worker",
                crate::mission::agent_profile::CapabilityPreset::HeadlessWriter,
                &[],
            ),
            worker_definition(
                "continuity-worker",
                crate::mission::agent_profile::CapabilityPreset::ReadOnlyReviewer,
                &[],
            ),
        ];

        let profile = select_worker_profile_for_feature(&feature, &worker_defs);

        assert_eq!(profile.name, "continuity-worker");
        assert_eq!(
            profile.capability_preset,
            crate::mission::agent_profile::CapabilityPreset::ReadOnlyReviewer
        );
    }

    #[test]
    fn select_worker_profile_falls_back_to_general_without_explicit_skill() {
        let feature = feature("feat-3", "Need a continuity audit for chapter 3", "");
        let worker_defs = vec![
            worker_definition(
                "draft-worker",
                crate::mission::agent_profile::CapabilityPreset::HeadlessWriter,
                &[],
            ),
            worker_definition(
                "general-worker",
                crate::mission::agent_profile::CapabilityPreset::ReadOnlyReviewer,
                &[],
            ),
        ];

        let profile = select_worker_profile_for_feature(&feature, &worker_defs);

        assert_eq!(profile.name, "general-worker");
        assert_eq!(
            profile.capability_preset,
            crate::mission::agent_profile::CapabilityPreset::ReadOnlyReviewer
        );
    }

    #[test]
    fn select_worker_profile_respects_forced_tool_overrides() {
        let feature = feature("feat-4", "Investigate setting notes", "researcher");
        let worker_defs = vec![worker_definition(
            "researcher",
            crate::mission::agent_profile::CapabilityPreset::SummaryOnly,
            &["todowrite"],
        )];

        let profile = select_worker_profile_for_feature(&feature, &worker_defs);

        assert_eq!(profile.name, "researcher");
        assert_eq!(
            profile.capability_preset,
            crate::mission::agent_profile::CapabilityPreset::SummaryOnly
        );
        assert_eq!(profile.forced_tools, vec!["todowrite".to_string()]);
    }

    #[test]
    fn feature_resource_locks_parses_explicit_lock_preconditions() {
        let mut feature = feature("feat-5", "Draft chapter", "writer");
        feature.preconditions = vec![
            "resource_lock:shared:chapter:vol1/ch1".to_string(),
            "resource_locks:file:chapters/ch1.md,canon:timeline".to_string(),
        ];

        let locks = feature_resource_locks(&feature);
        let keys = locks
            .iter()
            .map(|lock| lock.lock_id.clone())
            .collect::<std::collections::HashSet<_>>();

        assert!(keys.contains("chapter:vol1/ch1"));
        assert!(keys.contains("file:chapters/ch1.md"));
        assert!(keys.contains("canon:timeline"));
    }

    #[test]
    fn scheduling_locks_detect_shared_vs_exclusive_conflicts() {
        let shared_a = SchedulingLock {
            key: "chapter:vol1/ch1".to_string(),
            mode: ResourceLockMode::Shared,
        };
        let shared_b = SchedulingLock {
            key: "chapter:vol1/ch1".to_string(),
            mode: ResourceLockMode::Shared,
        };
        let exclusive = SchedulingLock {
            key: "chapter:vol1/ch1".to_string(),
            mode: ResourceLockMode::Exclusive,
        };

        assert!(!scheduling_locks_conflict(&shared_a, &shared_b));
        assert!(scheduling_locks_conflict(&shared_a, &exclusive));
    }

    #[test]
    fn select_schedulable_ready_features_skips_pending_feature_with_conflicting_lock() {
        let mut active = feature("feat-active", "Currently writing chapter 1", "writer");
        active.write_paths = vec!["chapters/ch1.md".to_string()];

        let mut conflicting = feature("feat-pending-conflict", "Also wants chapter 1", "writer");
        conflicting.write_paths = vec!["chapters/ch1.md".to_string()];

        let mut available = feature("feat-pending-ok", "Writes chapter 2", "writer");
        available.write_paths = vec!["chapters/ch2.md".to_string()];

        let features = vec![active.clone(), conflicting.clone(), available.clone()];
        let occupied_locks = scheduling_locks_for_feature(&active);
        let blockers: Vec<WorkflowBlocker> = Vec::new();

        let schedulable = select_schedulable_ready_features(
            vec![conflicting, available.clone()],
            &features,
            &blockers,
            occupied_locks,
            2,
        );

        let ids = schedulable
            .into_iter()
            .map(|feature| feature.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![available.id]);
    }

    #[test]
    fn evaluate_feature_schedulability_allows_shared_shared_but_blocks_shared_exclusive() {
        let mut active_shared = feature("feat-active-shared", "Read chapter 1", "reviewer");
        active_shared.preconditions = vec!["resource_lock:shared:chapter:vol1/ch1".to_string()];

        let mut candidate_shared =
            feature("feat-candidate-shared", "Also read chapter 1", "reviewer");
        candidate_shared.preconditions = vec!["resource_lock:shared:chapter:vol1/ch1".to_string()];

        let mut candidate_exclusive =
            feature("feat-candidate-exclusive", "Edit chapter 1", "writer");
        candidate_exclusive.preconditions = vec!["resource_lock:chapter:vol1/ch1".to_string()];

        let features = vec![
            active_shared.clone(),
            candidate_shared.clone(),
            candidate_exclusive.clone(),
        ];
        let occupied_locks = scheduling_locks_for_feature(&active_shared);
        let blockers: Vec<WorkflowBlocker> = Vec::new();
        let mut related_cache = HashMap::new();

        let shared_decision = evaluate_feature_schedulability_now(
            &candidate_shared,
            &features,
            &blockers,
            &occupied_locks,
            &mut related_cache,
        );
        assert!(matches!(
            shared_decision,
            FeatureSchedulability::Schedulable { .. }
        ));

        let exclusive_decision = evaluate_feature_schedulability_now(
            &candidate_exclusive,
            &features,
            &blockers,
            &occupied_locks,
            &mut related_cache,
        );
        assert_eq!(
            exclusive_decision,
            FeatureSchedulability::ExcludedNow(SchedulabilityExclusion::ResourceLockConflict)
        );
    }

    #[test]
    fn blocker_blocks_related_dependency_wave() {
        let mut blocked = feature("feat-a", "A", "writer");
        blocked.depends_on = Vec::new();
        let mut downstream = feature("feat-b", "B", "writer");
        downstream.depends_on = vec!["feat-a".to_string()];
        let unrelated = feature("feat-c", "C", "writer");
        let features = vec![blocked.clone(), downstream.clone(), unrelated.clone()];

        let blocker = crate::mission::blockers::WorkflowBlocker::external_dependency(
            "mis_1",
            "external gate",
            Some(blocked.id.clone()),
        );
        let mut cache = HashMap::new();

        assert!(blocker_blocks_feature(
            &blocker,
            &downstream,
            &features,
            &mut cache
        ));
        assert!(!blocker_blocks_feature(
            &blocker, &unrelated, &features, &mut cache
        ));
    }

    #[test]
    fn evaluate_feature_schedulability_excludes_downstream_feature_blocked_by_related_blocker() {
        let blocked = feature("feat-a", "A", "writer");
        let mut downstream = feature("feat-b", "B", "writer");
        downstream.depends_on = vec!["feat-a".to_string()];
        let unrelated = feature("feat-c", "C", "writer");
        let features = vec![blocked.clone(), downstream.clone(), unrelated];

        let blocker = WorkflowBlocker::external_dependency(
            "mis_1",
            "external gate",
            Some(blocked.id.clone()),
        );
        let blockers = vec![blocker];
        let mut related_cache = HashMap::new();

        let decision = evaluate_feature_schedulability_now(
            &downstream,
            &features,
            &blockers,
            &[],
            &mut related_cache,
        );

        assert_eq!(
            decision,
            FeatureSchedulability::ExcludedNow(SchedulabilityExclusion::BlockingBlocker)
        );
    }

    fn start_config(delegate_transport: DelegateTransportMode) -> MissionStartConfig {
        MissionStartConfig {
            run_config: MissionRunConfig {
                model: "m".to_string(),
                provider: "p".to_string(),
                base_url: "u".to_string(),
                api_key: "k".to_string(),
            },
            max_workers: 1,
            parent_session_id: None,
            parent_turn_id: None,
            delegate_transport,
        }
    }

    #[test]
    fn delegate_transport_mode_reads_process_from_start_config() {
        assert_eq!(
            delegate_transport_mode(&start_config(DelegateTransportMode::Process)),
            DelegateTransportMode::Process
        );
    }

    #[test]
    fn delegate_transport_mode_reads_in_process_from_start_config() {
        assert_eq!(
            delegate_transport_mode(&start_config(DelegateTransportMode::InProcess)),
            DelegateTransportMode::InProcess
        );
    }
}
