use std::fs;
use std::path::Path;

use crate::models::AppError;
use crate::services::versioning_port::EntityHead;

use super::core_head_wal::{append_wal, load_or_init_head_state, write_head_state_atomic};
use super::core_layout::{
    entity_path_from_entity_id, sanitize_entity_id, HEAD_FILE, SNAPSHOTS_DIR, WAL_FILE,
};
use super::core_types::{WalRecord, WalStatus, WalType};
use super::core_utils::new_tx_id;

const SNAPSHOT_EVERY_N_COMMITS: i64 = 20;
const SNAPSHOT_EVERY_MS: i64 = 10 * 60 * 1000;
const SNAPSHOT_WAL_SIZE_BYTES: u64 = 16 * 1024 * 1024;

fn snapshots_entity_dir(vc_root: &Path, entity_id: &str) -> std::path::PathBuf {
    vc_root
        .join(SNAPSHOTS_DIR)
        .join(sanitize_entity_id(entity_id))
}

fn snapshot_file_path(vc_root: &Path, entity_id: &str, revision: i64) -> std::path::PathBuf {
    snapshots_entity_dir(vc_root, entity_id).join(format!("rev_{:09}.json", revision))
}

pub(crate) fn maybe_snapshot(
    vc_root: &Path,
    project_path: &str,
    entity_id: &str,
    revision: i64,
    now_ms: i64,
) -> Result<(), AppError> {
    let head_state = load_or_init_head_state(vc_root, project_path)?;
    let head = head_state
        .entities
        .get(entity_id)
        .cloned()
        .unwrap_or_default();

    let entity_snap_dir = snapshots_entity_dir(vc_root, entity_id);
    if !entity_snap_dir.exists() {
        fs::create_dir_all(&entity_snap_dir)?;
    }

    if !should_create_snapshot(vc_root, revision, now_ms, head.last_snapshot_at)? {
        return Ok(());
    }

    let entity_path = entity_path_from_entity_id(project_path, entity_id)?;
    if !entity_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&entity_path)?;
    let snap_path = snapshot_file_path(vc_root, entity_id, revision);
    fs::write(&snap_path, content.as_bytes())?;

    append_snapshot_wal(
        vc_root,
        project_path,
        entity_id,
        revision,
        now_ms,
        &head,
        &snap_path,
    )
}

fn should_create_snapshot(
    vc_root: &Path,
    revision: i64,
    now_ms: i64,
    last_snapshot_at: Option<i64>,
) -> Result<bool, AppError> {
    if revision % SNAPSHOT_EVERY_N_COMMITS == 0 {
        return Ok(true);
    }

    if let Some(last) = last_snapshot_at {
        if now_ms - last >= SNAPSHOT_EVERY_MS {
            return Ok(true);
        }
    } else {
        return Ok(true);
    }

    let wal_path = vc_root.join(WAL_FILE);
    if wal_path.exists() {
        let sz = fs::metadata(&wal_path)?.len();
        if sz >= SNAPSHOT_WAL_SIZE_BYTES {
            return Ok(true);
        }
    }

    Ok(false)
}

fn append_snapshot_wal(
    vc_root: &Path,
    project_path: &str,
    entity_id: &str,
    revision: i64,
    now_ms: i64,
    head: &EntityHead,
    snap_path: &Path,
) -> Result<(), AppError> {
    let mut head_state = load_or_init_head_state(vc_root, project_path)?;
    if let Some(h) = head_state.entities.get_mut(entity_id) {
        h.last_snapshot_at = Some(now_ms);
    }

    let event_seq = head_state.last_event_seq + 1;
    head_state.last_event_seq = event_seq;
    write_head_state_atomic(&vc_root.join(HEAD_FILE), &head_state)?;

    append_wal(
        &vc_root.join(WAL_FILE),
        &WalRecord {
            event_seq,
            ts: now_ms,
            r#type: WalType::Snapshot,
            tx_id: new_tx_id(),
            call_id: None,
            actor: None,
            entity_id: entity_id.to_string(),
            from_revision: Some(revision),
            to_revision: Some(revision),
            expected_revision: None,
            before_hash: None,
            after_hash: Some(head.json_hash.clone()),
            patch_hash: None,
            status: WalStatus::Ok,
            detail: Some(snap_path.to_string_lossy().to_string()),
        },
    )
}

pub(crate) fn reconstruct_entity_at_revision(
    vc_root: &Path,
    project_path: &str,
    entity_id: &str,
    target_revision: i64,
) -> Result<serde_json::Value, AppError> {
    if let Some(json) =
        super::core_revision::read_revision_record(vc_root, entity_id, target_revision)?
    {
        return Ok(json);
    }

    if target_revision == 0 {
        return Ok(serde_json::json!({"type":"doc","content":[]}));
    }

    let entity_path = entity_path_from_entity_id(project_path, entity_id)?;
    if entity_path.exists() {
        let s = fs::read_to_string(&entity_path)?;
        return serde_json::from_str(&s).map_err(|e| {
            super::core_utils::app_err_vc("E_VC_RECOVERY_REQUIRED", e.to_string(), false)
        });
    }

    Err(super::core_utils::app_err_vc(
        "E_VC_RECOVERY_REQUIRED",
        "revision record not found".to_string(),
        false,
    ))
}
