use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{AppError, ErrorCode};
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;

use super::contract::{prepare_events_for_append, EventContractState};
use super::{
    find_meta, load_index, migrate_event, read_events_jsonl, rebuild_index_for_session,
    session_index_path, sessions_root, write_events_jsonl, AgentSessionEvent,
    AGENT_SESSION_SCHEMA_VERSION,
};

pub const SESSION_MIGRATION_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMigrationSessionReport {
    pub session_id: String,
    pub stream_path: String,
    pub total_events: usize,
    pub legacy_event_count: usize,
    pub missing_event_seq_count: usize,
    pub non_monotonic_event_seq_count: usize,
    pub missing_dedupe_key_count: usize,
    pub session_id_mismatch_count: usize,
    pub index_missing: bool,
    pub index_last_turn_mismatch: bool,
    pub index_last_stop_reason_mismatch: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anomaly_categories: Vec<String>,
    pub quarantine_recommended: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_repair_actions: Vec<String>,
    pub needs_migration: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMigrationManualRepairItem {
    pub session_id: String,
    pub categories: Vec<String>,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMigrationDryRunOutput {
    pub schema_version: i32,
    pub migration_id: String,
    pub scanned_sessions: usize,
    pub sessions_needing_migration: usize,
    pub quarantined_sessions: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_repair_queue: Vec<SessionMigrationManualRepairItem>,
    pub reports: Vec<SessionMigrationSessionReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMigrationFailure {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMigrationBatchOutput {
    pub batch_id: u32,
    pub migrated_sessions: usize,
    pub migrated_events: usize,
    pub sessions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMigrationCommitOutput {
    pub schema_version: i32,
    pub migration_id: String,
    pub backup_root: String,
    pub migrated_sessions: usize,
    pub migrated_events: usize,
    pub skipped_sessions: usize,
    pub batches: Vec<SessionMigrationBatchOutput>,
    pub failed_sessions: Vec<SessionMigrationFailure>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quarantined_sessions: Vec<SessionMigrationFailure>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_repair_queue: Vec<SessionMigrationManualRepairItem>,
    pub version_marker_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMigrationRollbackOutput {
    pub schema_version: i32,
    pub migration_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<u32>,
    pub restored_streams: usize,
    pub restored_index: bool,
    pub restored_version_marker: bool,
}

#[derive(Debug, Clone)]
struct SessionTarget {
    session_id: String,
    stream_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationManifest {
    schema_version: i32,
    migration_id: String,
    project_path: String,
    created_at: i64,
    batch_size: usize,
    batches: Vec<BatchManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    index_backup_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version_marker_backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchManifest {
    batch_id: u32,
    items: Vec<BatchMigrationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchMigrationItem {
    session_id: String,
    original_stream_path: String,
    backup_stream_path: String,
    events_before: usize,
    events_after: usize,
    deduped_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationVersionMarker {
    schema_version: i32,
    migration_id: String,
    updated_at: i64,
    migrated_sessions: usize,
}

pub fn dry_run_session_migration(
    project_path: &Path,
    session_ids: Option<&[String]>,
) -> Result<SessionMigrationDryRunOutput, AppError> {
    let targets = collect_targets(project_path, session_ids)?;
    let mut reports = Vec::with_capacity(targets.len());
    let index = load_index(&session_index_path(project_path)).ok();

    for target in targets {
        let index_meta = index
            .as_ref()
            .and_then(|loaded| find_meta(loaded, &target.session_id));
        reports.push(inspect_target(&target, index_meta.as_ref()));
    }

    let sessions_needing_migration = reports
        .iter()
        .filter(|report| report.needs_migration)
        .count();
    let quarantined_sessions = reports
        .iter()
        .filter(|report| report.quarantine_recommended)
        .count();
    let manual_repair_queue = reports
        .iter()
        .filter(|report| !report.manual_repair_actions.is_empty())
        .map(|report| SessionMigrationManualRepairItem {
            session_id: report.session_id.clone(),
            categories: report.anomaly_categories.clone(),
            actions: report.manual_repair_actions.clone(),
        })
        .collect();

    Ok(SessionMigrationDryRunOutput {
        schema_version: SESSION_MIGRATION_SCHEMA_VERSION,
        migration_id: format!(
            "session_migration_dry_run_{}_{}",
            Utc::now().format("%Y%m%d%H%M%S"),
            Uuid::new_v4().simple()
        ),
        scanned_sessions: reports.len(),
        sessions_needing_migration,
        quarantined_sessions,
        manual_repair_queue,
        reports,
    })
}

pub fn commit_session_migration(
    project_path: &Path,
    session_ids: Option<&[String]>,
    batch_size: usize,
) -> Result<SessionMigrationCommitOutput, AppError> {
    let batch_size = batch_size.max(1);
    let targets = collect_targets(project_path, session_ids)?;
    let index = load_index(&session_index_path(project_path)).ok();
    let reports = targets
        .iter()
        .map(|target| {
            let index_meta = index
                .as_ref()
                .and_then(|loaded| find_meta(loaded, &target.session_id));
            inspect_target(target, index_meta.as_ref())
        })
        .collect::<Vec<SessionMigrationSessionReport>>();

    let migration_id = format!(
        "session_migration_{}_{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        Uuid::new_v4().simple()
    );

    let backup_root = migration_backups_root(project_path).join(&migration_id);
    ensure_dir(&backup_root)?;

    let index_backup_path = backup_if_exists(
        &session_index_path(project_path),
        &backup_root.join("index.json.bak"),
    )?;
    let version_marker_path = migration_version_marker_path(project_path);
    let version_marker_backup_path = backup_if_exists(
        &version_marker_path,
        &backup_root.join("version-marker.json.bak"),
    )?;

    let mut manifest_batches = Vec::new();
    let mut output_batches = Vec::new();
    let mut migrated_sessions = 0_usize;
    let mut migrated_events = 0_usize;
    let mut skipped_sessions = 0_usize;
    let mut failures = Vec::new();
    let mut quarantined_sessions = Vec::new();

    let mut migration_targets = Vec::new();
    for target in &targets {
        let report = reports
            .iter()
            .find(|report| report.session_id == target.session_id)
            .cloned()
            .unwrap_or_else(|| {
                let index_meta = index
                    .as_ref()
                    .and_then(|loaded| find_meta(loaded, &target.session_id));
                inspect_target(target, index_meta.as_ref())
            });
        if report.needs_migration {
            if report.quarantine_recommended {
                skipped_sessions = skipped_sessions.saturating_add(1);
                quarantined_sessions.push(SessionMigrationFailure {
                    session_id: target.session_id.clone(),
                    message: if report.manual_repair_actions.is_empty() {
                        "quarantined due to migration consistency anomaly".to_string()
                    } else {
                        format!(
                            "quarantined for manual repair: {}",
                            report.manual_repair_actions.join("; ")
                        )
                    },
                });
            } else {
                migration_targets.push(target.clone());
            }
        } else {
            skipped_sessions = skipped_sessions.saturating_add(1);
        }
    }

    let manual_repair_queue = reports
        .iter()
        .filter(|report| !report.manual_repair_actions.is_empty())
        .map(|report| SessionMigrationManualRepairItem {
            session_id: report.session_id.clone(),
            categories: report.anomaly_categories.clone(),
            actions: report.manual_repair_actions.clone(),
        })
        .collect::<Vec<SessionMigrationManualRepairItem>>();

    for (batch_index, batch_targets) in migration_targets.chunks(batch_size).enumerate() {
        let batch_id = (batch_index as u32).saturating_add(1);
        let batch_dir = backup_root.join(format!("batch_{batch_id:04}"));
        ensure_dir(&batch_dir)?;

        let mut batch_items = Vec::new();
        let mut batch_sessions = Vec::new();
        let mut batch_migrated_events = 0_usize;

        for target in batch_targets {
            let backup_stream_path = batch_dir.join(format!("{}.jsonl.bak", target.session_id));

            if let Err(err) = copy_file(&target.stream_path, &backup_stream_path) {
                failures.push(SessionMigrationFailure {
                    session_id: target.session_id.clone(),
                    message: err.message,
                });
                continue;
            }

            let original_events = match read_events_jsonl(&target.stream_path) {
                Ok(events) => events,
                Err(err) => {
                    failures.push(SessionMigrationFailure {
                        session_id: target.session_id.clone(),
                        message: err.message,
                    });
                    continue;
                }
            };

            let mut migrated = original_events.clone();
            for event in &mut migrated {
                migrate_event(event);
            }

            let prepared = match prepare_events_for_append(
                &target.session_id,
                &migrated,
                EventContractState::default(),
            ) {
                Ok(prepared) => prepared,
                Err(err) => {
                    failures.push(SessionMigrationFailure {
                        session_id: target.session_id.clone(),
                        message: err.message,
                    });
                    continue;
                }
            };

            if let Err(err) = write_events_jsonl(&target.stream_path, &prepared.events) {
                let _ = restore_file(&backup_stream_path, &target.stream_path);
                failures.push(SessionMigrationFailure {
                    session_id: target.session_id.clone(),
                    message: err.message,
                });
                continue;
            }

            if let Err(err) =
                rebuild_index_for_session(project_path, &target.session_id, &prepared.events)
            {
                let _ = restore_file(&backup_stream_path, &target.stream_path);
                failures.push(SessionMigrationFailure {
                    session_id: target.session_id.clone(),
                    message: err.message,
                });
                continue;
            }

            migrated_sessions = migrated_sessions.saturating_add(1);
            migrated_events = migrated_events.saturating_add(prepared.events.len());
            batch_migrated_events = batch_migrated_events.saturating_add(prepared.events.len());
            batch_sessions.push(target.session_id.clone());
            batch_items.push(BatchMigrationItem {
                session_id: target.session_id.clone(),
                original_stream_path: target.stream_path.to_string_lossy().to_string(),
                backup_stream_path: backup_stream_path.to_string_lossy().to_string(),
                events_before: original_events.len(),
                events_after: prepared.events.len(),
                deduped_count: prepared.deduped_count,
            });
        }

        if !batch_items.is_empty() {
            output_batches.push(SessionMigrationBatchOutput {
                batch_id,
                migrated_sessions: batch_items.len(),
                migrated_events: batch_migrated_events,
                sessions: batch_sessions,
            });

            manifest_batches.push(BatchManifest {
                batch_id,
                items: batch_items,
            });
        }
    }

    let manifest = MigrationManifest {
        schema_version: SESSION_MIGRATION_SCHEMA_VERSION,
        migration_id: migration_id.clone(),
        project_path: project_path.to_string_lossy().to_string(),
        created_at: Utc::now().timestamp_millis(),
        batch_size,
        batches: manifest_batches,
        index_backup_path: index_backup_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        version_marker_backup_path: version_marker_backup_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
    };

    let manifest_path = backup_root.join("manifest.json");
    atomic_write_json(&manifest_path, &manifest).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write migration manifest: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_MIGRATION_MANIFEST_WRITE_FAILED",
            "path": manifest_path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    let marker = MigrationVersionMarker {
        schema_version: AGENT_SESSION_SCHEMA_VERSION,
        migration_id: migration_id.clone(),
        updated_at: Utc::now().timestamp_millis(),
        migrated_sessions,
    };

    atomic_write_json(&version_marker_path, &marker).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write migration version marker: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_MIGRATION_VERSION_WRITE_FAILED",
            "path": version_marker_path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    Ok(SessionMigrationCommitOutput {
        schema_version: SESSION_MIGRATION_SCHEMA_VERSION,
        migration_id,
        backup_root: backup_root.to_string_lossy().to_string(),
        migrated_sessions,
        migrated_events,
        skipped_sessions,
        batches: output_batches,
        failed_sessions: failures,
        quarantined_sessions,
        manual_repair_queue,
        version_marker_path: version_marker_path.to_string_lossy().to_string(),
    })
}

pub fn rollback_session_migration(
    project_path: &Path,
    migration_id: &str,
    batch_id: Option<u32>,
) -> Result<SessionMigrationRollbackOutput, AppError> {
    let backup_root = migration_backups_root(project_path).join(migration_id);
    let manifest_path = backup_root.join("manifest.json");

    if !manifest_path.exists() {
        return Err(AppError {
            code: ErrorCode::NotFound,
            message: "migration manifest not found".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_MIGRATION_MANIFEST_NOT_FOUND",
                "migration_id": migration_id,
                "path": manifest_path.to_string_lossy(),
            })),
            recoverable: Some(false),
        });
    }

    let manifest: MigrationManifest = read_json(&manifest_path).map_err(|err| AppError {
        code: ErrorCode::JsonParseError,
        message: format!("failed to parse migration manifest: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_MIGRATION_MANIFEST_PARSE_FAILED",
            "path": manifest_path.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    let mut restored_streams = 0_usize;

    for batch in &manifest.batches {
        if batch_id.is_some() && Some(batch.batch_id) != batch_id {
            continue;
        }

        for item in &batch.items {
            restore_file(
                Path::new(&item.backup_stream_path),
                Path::new(&item.original_stream_path),
            )?;

            let restored_events = read_events_jsonl(Path::new(&item.original_stream_path))?;
            rebuild_index_for_session(project_path, &item.session_id, &restored_events)?;
            restored_streams = restored_streams.saturating_add(1);
        }
    }

    let mut restored_index = false;
    let mut restored_version_marker = false;

    if batch_id.is_none() {
        if let Some(index_backup_path) = manifest.index_backup_path.as_deref() {
            restore_file(
                Path::new(index_backup_path),
                &session_index_path(project_path),
            )?;
            restored_index = true;
        }

        if let Some(marker_backup_path) = manifest.version_marker_backup_path.as_deref() {
            restore_file(
                Path::new(marker_backup_path),
                &migration_version_marker_path(project_path),
            )?;
            restored_version_marker = true;
        }
    }

    Ok(SessionMigrationRollbackOutput {
        schema_version: SESSION_MIGRATION_SCHEMA_VERSION,
        migration_id: migration_id.to_string(),
        batch_id,
        restored_streams,
        restored_index,
        restored_version_marker,
    })
}

fn collect_targets(
    project_path: &Path,
    session_ids: Option<&[String]>,
) -> Result<Vec<SessionTarget>, AppError> {
    let root = sessions_root(project_path);
    ensure_dir(&root)?;

    if let Some(session_ids) = session_ids {
        let mut targets = Vec::with_capacity(session_ids.len());
        for session_id in session_ids {
            let session_id = session_id.trim();
            if session_id.is_empty() {
                continue;
            }

            let stream_path = root.join(format!("{session_id}.jsonl"));
            if !stream_path.exists() {
                return Err(AppError {
                    code: ErrorCode::NotFound,
                    message: "session stream not found for migration".to_string(),
                    details: Some(serde_json::json!({
                        "code": "E_AGENT_SESSION_MIGRATION_SESSION_NOT_FOUND",
                        "session_id": session_id,
                        "path": stream_path.to_string_lossy(),
                    })),
                    recoverable: Some(false),
                });
            }

            targets.push(SessionTarget {
                session_id: session_id.to_string(),
                stream_path,
            });
        }

        targets.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        return Ok(targets);
    }

    let mut targets = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to list session streams for migration: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_MIGRATION_LIST_FAILED",
            "path": root.to_string_lossy(),
        })),
        recoverable: Some(true),
    })? {
        let entry = entry.map_err(|err| AppError {
            code: ErrorCode::IoError,
            message: format!("failed to read session stream entry for migration: {err}"),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_MIGRATION_LIST_FAILED",
                "path": root.to_string_lossy(),
            })),
            recoverable: Some(true),
        })?;

        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }

        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };

        targets.push(SessionTarget {
            session_id: stem.to_string(),
            stream_path: path,
        });
    }

    targets.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(targets)
}

fn normalize_stop_reason(value: Option<&str>) -> Option<String> {
    value.map(str::trim).and_then(|raw| match raw {
        "success" | "cancel" | "error" | "limit" => Some(raw.to_string()),
        _ => None,
    })
}

fn derive_stream_last_turn(events: &[AgentSessionEvent]) -> Option<i64> {
    let mut last_turn = None;
    for event in events {
        if let Some(turn) = event.turn {
            if turn > 0 && last_turn.map_or(true, |current| turn > current) {
                last_turn = Some(turn);
            }
        }
    }
    last_turn
}

fn derive_stream_last_stop_reason(events: &[AgentSessionEvent]) -> Option<String> {
    let mut reason = None;

    for event in events {
        let from_payload = event
            .payload
            .as_ref()
            .and_then(|payload| payload.get("stop_reason"))
            .and_then(|value| value.as_str());

        match event.event_type.as_str() {
            "turn_completed" => {
                reason =
                    normalize_stop_reason(from_payload).or_else(|| Some("success".to_string()));
            }
            "turn_failed" => {
                reason = normalize_stop_reason(from_payload).or_else(|| Some("error".to_string()));
            }
            "turn_cancelled" => {
                reason = normalize_stop_reason(from_payload).or_else(|| Some("cancel".to_string()));
            }
            _ => {}
        }
    }

    reason
}

fn inspect_target(
    target: &SessionTarget,
    index_meta: Option<&super::AgentSessionMeta>,
) -> SessionMigrationSessionReport {
    match read_events_jsonl(&target.stream_path) {
        Ok(events) => {
            let mut legacy_event_count = 0_usize;
            let mut missing_event_seq_count = 0_usize;
            let mut non_monotonic_event_seq_count = 0_usize;
            let mut missing_dedupe_key_count = 0_usize;
            let mut session_id_mismatch_count = 0_usize;
            let mut last_event_seq = 0_i64;

            for event in &events {
                if event.schema_version != AGENT_SESSION_SCHEMA_VERSION {
                    legacy_event_count = legacy_event_count.saturating_add(1);
                }

                if event.session_id.trim() != target.session_id {
                    session_id_mismatch_count = session_id_mismatch_count.saturating_add(1);
                }

                match event.event_seq {
                    Some(event_seq) if event_seq > 0 => {
                        if event_seq <= last_event_seq {
                            non_monotonic_event_seq_count =
                                non_monotonic_event_seq_count.saturating_add(1);
                        } else {
                            last_event_seq = event_seq;
                        }
                    }
                    _ => {
                        missing_event_seq_count = missing_event_seq_count.saturating_add(1);
                    }
                }

                let missing_dedupe = event
                    .dedupe_key
                    .as_ref()
                    .map(|value| value.trim().is_empty())
                    .unwrap_or(true);

                if missing_dedupe {
                    missing_dedupe_key_count = missing_dedupe_key_count.saturating_add(1);
                }
            }

            let derived_last_turn = derive_stream_last_turn(&events);
            let derived_last_stop_reason = derive_stream_last_stop_reason(&events);
            let index_missing = index_meta.is_none();
            let index_last_turn_mismatch = index_meta
                .map(|meta| meta.last_turn != derived_last_turn)
                .unwrap_or(false);
            let index_last_stop_reason_mismatch = index_meta
                .map(|meta| {
                    let meta_reason = normalize_stop_reason(meta.last_stop_reason.as_deref());
                    meta_reason != derived_last_stop_reason
                })
                .unwrap_or(false);

            let mut anomaly_categories = Vec::new();
            let mut manual_repair_actions = Vec::new();

            if legacy_event_count > 0 {
                anomaly_categories.push("legacy_schema_event".to_string());
            }
            if missing_event_seq_count > 0 {
                anomaly_categories.push("missing_event_seq".to_string());
            }
            if non_monotonic_event_seq_count > 0 {
                anomaly_categories.push("non_monotonic_event_seq".to_string());
            }
            if missing_dedupe_key_count > 0 {
                anomaly_categories.push("missing_dedupe_key".to_string());
            }
            if session_id_mismatch_count > 0 {
                anomaly_categories.push("session_id_mismatch".to_string());
                manual_repair_actions.push(
                    "quarantine session and repair stream event session_id before migration"
                        .to_string(),
                );
            }
            if index_missing {
                anomaly_categories.push("index_missing".to_string());
                manual_repair_actions.push(
                    "rebuild index entry from stream snapshot before canary cutover".to_string(),
                );
            }
            if index_last_turn_mismatch {
                anomaly_categories.push("index_last_turn_mismatch".to_string());
                manual_repair_actions
                    .push("verify index.last_turn and rebuild index from stream".to_string());
            }
            if index_last_stop_reason_mismatch {
                anomaly_categories.push("index_last_stop_reason_mismatch".to_string());
                manual_repair_actions.push(
                    "verify index.last_stop_reason and rebuild index from stream".to_string(),
                );
            }

            let quarantine_recommended = session_id_mismatch_count > 0;

            let needs_migration = legacy_event_count > 0
                || missing_event_seq_count > 0
                || non_monotonic_event_seq_count > 0
                || missing_dedupe_key_count > 0
                || session_id_mismatch_count > 0
                || index_missing
                || index_last_turn_mismatch
                || index_last_stop_reason_mismatch;

            SessionMigrationSessionReport {
                session_id: target.session_id.clone(),
                stream_path: target.stream_path.to_string_lossy().to_string(),
                total_events: events.len(),
                legacy_event_count,
                missing_event_seq_count,
                non_monotonic_event_seq_count,
                missing_dedupe_key_count,
                session_id_mismatch_count,
                index_missing,
                index_last_turn_mismatch,
                index_last_stop_reason_mismatch,
                anomaly_categories,
                quarantine_recommended,
                manual_repair_actions,
                needs_migration,
                error: None,
            }
        }
        Err(err) => SessionMigrationSessionReport {
            session_id: target.session_id.clone(),
            stream_path: target.stream_path.to_string_lossy().to_string(),
            total_events: 0,
            legacy_event_count: 0,
            missing_event_seq_count: 0,
            non_monotonic_event_seq_count: 0,
            missing_dedupe_key_count: 0,
            session_id_mismatch_count: 0,
            index_missing: index_meta.is_none(),
            index_last_turn_mismatch: false,
            index_last_stop_reason_mismatch: false,
            anomaly_categories: vec!["stream_read_error".to_string()],
            quarantine_recommended: true,
            manual_repair_actions: vec![
                "run agent_session_recover then inspect stream manually before migration"
                    .to_string(),
            ],
            needs_migration: true,
            error: Some(err.message),
        },
    }
}

fn migration_backups_root(project_path: &Path) -> PathBuf {
    sessions_root(project_path).join(".migration_backups")
}

fn migration_version_marker_path(project_path: &Path) -> PathBuf {
    sessions_root(project_path).join("session_migration_version.json")
}

fn backup_if_exists(source: &Path, backup: &Path) -> Result<Option<PathBuf>, AppError> {
    if !source.exists() {
        return Ok(None);
    }

    copy_file(source, backup)?;
    Ok(Some(backup.to_path_buf()))
}

fn copy_file(source: &Path, target: &Path) -> Result<(), AppError> {
    let parent = target.parent().ok_or_else(|| AppError {
        code: ErrorCode::InvalidArgument,
        message: "invalid target path for file copy".to_string(),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_MIGRATION_COPY_INVALID_TARGET",
            "target": target.to_string_lossy(),
        })),
        recoverable: Some(false),
    })?;

    ensure_dir(parent)?;

    std::fs::copy(source, target).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to copy migration file: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_MIGRATION_COPY_FAILED",
            "source": source.to_string_lossy(),
            "target": target.to_string_lossy(),
        })),
        recoverable: Some(true),
    })?;

    Ok(())
}

fn restore_file(source: &Path, target: &Path) -> Result<(), AppError> {
    copy_file(source, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_session::{
        append_events_jsonl, session_stream_path, AgentSessionEvent,
    };

    fn setup_temp_project() -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("magic_session_migration_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(base.join("magic_novel").join("ai").join("sessions")).unwrap();
        base
    }

    fn make_event(
        session_id: &str,
        schema_version: i32,
        event_seq: Option<i64>,
        dedupe_key: Option<&str>,
    ) -> AgentSessionEvent {
        AgentSessionEvent {
            schema_version,
            event_type: "turn_started".to_string(),
            session_id: session_id.to_string(),
            ts: Utc::now().timestamp_millis(),
            event_id: Some(format!("evt_{}", Uuid::new_v4())),
            event_seq,
            dedupe_key: dedupe_key.map(|value| value.to_string()),
            turn: Some(1),
            payload: None,
        }
    }

    #[test]
    fn dry_run_detects_legacy_and_contract_gaps() {
        let project = setup_temp_project();
        let session_id = "migration_dry_run";
        let stream = session_stream_path(project.as_path(), session_id);

        append_events_jsonl(
            &stream,
            &[
                make_event(session_id, 0, Some(1), None),
                make_event(session_id, 1, None, Some("")),
            ],
        )
        .unwrap();

        let report = dry_run_session_migration(project.as_path(), None).unwrap();
        assert_eq!(report.scanned_sessions, 1);
        assert_eq!(report.sessions_needing_migration, 1);
        assert_eq!(report.quarantined_sessions, 0);
        assert_eq!(report.reports[0].legacy_event_count, 1);
        assert_eq!(report.reports[0].missing_event_seq_count, 1);
        assert_eq!(report.reports[0].missing_dedupe_key_count, 2);
        assert!(report.reports[0].index_missing);
        assert!(!report.reports[0].manual_repair_actions.is_empty());
        assert_eq!(report.manual_repair_queue.len(), 1);
    }

    #[test]
    fn commit_and_rollback_roundtrip() {
        let project = setup_temp_project();
        let session_id = "migration_commit";
        let stream = session_stream_path(project.as_path(), session_id);

        let original = vec![
            make_event(session_id, 0, Some(1), None),
            make_event(session_id, 1, None, None),
        ];
        append_events_jsonl(&stream, &original).unwrap();

        let commit = commit_session_migration(project.as_path(), None, 50).unwrap();
        assert_eq!(commit.migrated_sessions, 1);
        assert!(Path::new(&commit.version_marker_path).exists());
        assert!(commit.quarantined_sessions.is_empty());
        assert_eq!(commit.manual_repair_queue.len(), 1);

        let migrated = read_events_jsonl(&stream).unwrap();
        assert!(migrated
            .iter()
            .all(|event| event.schema_version == AGENT_SESSION_SCHEMA_VERSION));
        assert!(migrated.iter().all(|event| event.event_seq.is_some()));
        assert!(migrated.iter().all(|event| {
            event
                .dedupe_key
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        }));

        let rollback =
            rollback_session_migration(project.as_path(), &commit.migration_id, None).unwrap();
        assert_eq!(rollback.restored_streams, 1);

        let restored = read_events_jsonl(&stream).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].schema_version, 0);
        assert!(restored[1].event_seq.is_none());
    }

    #[test]
    fn dry_run_and_commit_quarantine_session_id_mismatch() {
        let project = setup_temp_project();
        let session_id = "migration_quarantine";
        let stream = session_stream_path(project.as_path(), session_id);

        append_events_jsonl(
            &stream,
            &[make_event("another_session", 1, Some(1), Some("dup_1"))],
        )
        .unwrap();

        let dry_run = dry_run_session_migration(project.as_path(), None).unwrap();
        assert_eq!(dry_run.scanned_sessions, 1);
        assert_eq!(dry_run.quarantined_sessions, 1);
        assert_eq!(dry_run.reports[0].session_id_mismatch_count, 1);
        assert!(dry_run.reports[0]
            .anomaly_categories
            .iter()
            .any(|item| item == "session_id_mismatch"));

        let commit = commit_session_migration(project.as_path(), None, 20).unwrap();
        assert_eq!(commit.migrated_sessions, 0);
        assert_eq!(commit.skipped_sessions, 1);
        assert_eq!(commit.quarantined_sessions.len(), 1);
        assert_eq!(commit.quarantined_sessions[0].session_id, session_id);
    }
}
