use std::sync::Arc;

use tokio::sync::Mutex as TokioMutex;

use crate::mission::artifacts;
use crate::mission::events::MissionEventEmitter;
use crate::mission::orchestrator::Orchestrator;
use crate::mission::process_manager::{ProcessManager, WorkerProcess};
use crate::mission::types::INTEGRATOR_FEATURE_ID;
use crate::mission::types::*;
use crate::mission::worker_profile::{
    builtin_general_worker_profile, builtin_integrator_worker_profile, WorkerProfile,
    WorkerProfileSummary, WorkerRunEntry,
};
use crate::mission::worker_protocol::StartFeaturePayload;
use crate::models::AppError;
use crate::services::global_config;

use super::runtime::*;
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

    let recent_handoffs = handoffs
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
        .collect::<Vec<_>>();

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
        "Read mission artifacts and compile final handoff summary".to_string(),
        "Ensure unresolved items are explicit and actionable".to_string(),
        if recent_handoffs.is_empty() {
            "No handoffs available yet".to_string()
        } else {
            format!("Recent handoffs:\n{}", recent_handoffs.join("\n"))
        },
    ];

    Ok(())
}

pub(super) fn select_worker_profile_for_feature(
    feature: &Feature,
    worker_defs: &[global_config::WorkerDefinition],
) -> WorkerProfile {
    let skill = feature.skill.trim();
    if !skill.is_empty() {
        if let Some(def) = worker_defs
            .iter()
            .find(|d| d.name.trim().eq_ignore_ascii_case(skill))
        {
            let mut profile = WorkerProfile::from_definition(def);
            if profile.tool_whitelist.is_empty() {
                profile.tool_whitelist = builtin_general_worker_profile().tool_whitelist;
            }
            return profile;
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
            let mut profile = WorkerProfile::from_definition(def);
            if profile.tool_whitelist.is_empty() {
                profile.tool_whitelist = builtin_integrator_worker_profile().tool_whitelist;
            }
            return profile;
        }
        return builtin_integrator_worker_profile();
    }

    let desc = feature.description.to_lowercase();
    let mut best: Option<(&global_config::WorkerDefinition, usize)> = None;
    for def in worker_defs {
        if def.match_keywords.is_empty() {
            continue;
        }
        let matches = def
            .match_keywords
            .iter()
            .map(|k| k.trim().to_lowercase())
            .filter(|k| !k.is_empty())
            .filter(|k| desc.contains(k))
            .count();
        if matches == 0 {
            continue;
        }

        if best
            .as_ref()
            .map(|(_, best_matches)| matches > *best_matches)
            .unwrap_or(true)
        {
            best = Some((def, matches));
        }
    }

    if let Some((def, _)) = best {
        let mut profile = WorkerProfile::from_definition(def);
        if profile.tool_whitelist.is_empty() {
            profile.tool_whitelist = builtin_general_worker_profile().tool_whitelist;
        }
        return profile;
    }

    // Prefer conventional default worker names when present.
    for name in ["general-worker", "draft-worker", "general", "draft"] {
        if let Some(def) = worker_defs
            .iter()
            .find(|d| d.name.trim().eq_ignore_ascii_case(name))
        {
            let mut profile = WorkerProfile::from_definition(def);
            if profile.tool_whitelist.is_empty() {
                profile.tool_whitelist = builtin_general_worker_profile().tool_whitelist;
            }
            return profile;
        }
    }

    builtin_general_worker_profile()
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

pub(super) async fn spawn_and_initialize_worker(
    project_path: &std::path::Path,
    project_path_str: &str,
    mission_id: &str,
) -> Result<(String, Arc<TokioMutex<WorkerProcess>>), AppError> {
    let worker_binary = ProcessManager::find_worker_binary()?;
    let mission_dir = artifacts::mission_dir(project_path, mission_id)
        .to_string_lossy()
        .to_string();
    let worker_id = format!("wk_{}", uuid::Uuid::new_v4());
    let pm = ProcessManager::new(worker_binary);
    let mut worker = pm.spawn(&worker_id)?;

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

    Ok((worker_id, Arc::new(TokioMutex::new(worker))))
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
    let active_workers = list_worker_handles(mission_id).len();
    if active_workers >= start_config.max_workers {
        return Ok(Vec::new());
    }

    let available_slots = start_config.max_workers.saturating_sub(active_workers);
    if available_slots == 0 {
        return Ok(Vec::new());
    }

    let ready_features = orch.ready_pending_features(available_slots)?;
    if ready_features.is_empty() {
        return Ok(Vec::new());
    }

    let worker_defs = global_config::load_worker_definitions();

    let mut started = Vec::new();

    for (idx, feature) in ready_features.into_iter().enumerate() {
        let mut feature = feature;
        let _ = enrich_integrator_feature_context(project_path, mission_id, &mut feature);

        let worker_profile = select_worker_profile_for_feature(&feature, &worker_defs);

        let (worker_id, worker_arc) =
            spawn_and_initialize_worker(project_path, project_path_str, mission_id).await?;

        let attempt = 0_u32;
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

                super::supervision::spawn_worker_supervision_tasks(
                    app_handle.clone(),
                    mission_id.to_string(),
                    project_path_str.to_string(),
                    worker_id,
                    start_config.run_config.clone(),
                    start_config.max_workers,
                );

                started.push(feature_id);
            }
            Err(e) => {
                clear_worker_from_state(project_path, mission_id, &worker_id);
                rollback_active_feature_to_pending(
                    orch,
                    project_path,
                    mission_id,
                    Some(&worker_id),
                );
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
    worker_profile: WorkerProfile,
    rollback_to_pending_on_start_error: bool,
    emit_orchestrator_transition: bool,
) -> Result<String, AppError> {
    let session_id = format!(
        "worker_{}_{}_{}_{}",
        mission_id,
        feature.id,
        attempt,
        chrono::Utc::now().timestamp_millis()
    );

    let effective_model = worker_profile
        .model
        .as_deref()
        .unwrap_or(run_config.model.as_str())
        .trim()
        .to_string();
    let profile_summary = WorkerProfileSummary::from_profile(&worker_profile);

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
        .start_feature(StartFeaturePayload {
            feature: feature.clone(),
            session_id,
            model: effective_model,
            provider: run_config.provider.clone(),
            base_url: run_config.base_url.clone(),
            api_key: run_config.api_key.clone(),
            mission_id: mission_id.to_string(),
            worker_id: worker_id.to_string(),
            worker_profile: Some(worker_profile),
            parent_session_id: None,
            parent_turn_id: None,
        })
        .await
    {
        if rollback_to_pending_on_start_error {
            let _ = orch.update_feature_status(&feature.id, FeatureStatus::Pending);
        }
        return Err(e);
    }

    Ok(feature.id)
}
