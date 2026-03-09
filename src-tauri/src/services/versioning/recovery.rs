use crate::models::AppError;
use crate::services::versioning_port::{RecoverOutput, VcActor};

use super::core::{
    append_wal, cleanup_tmp_files, ensure_vc_layout, next_event_seq, rebuild_call_index_from_wal,
    rebuild_head_from_wal, truncate_wal_if_corrupted, vc_root_dir, WAL_FILE,
};
use super::core_types::{WalRecord, WalStatus, WalType};

const RECOVERY_ENTITY_ID: &str = "recovery:project";

pub fn recover_project(project_path: &str) -> Result<RecoverOutput, AppError> {
    let vc_root = vc_root_dir(project_path);
    ensure_vc_layout(&vc_root)?;

    let mut out = RecoverOutput::default();
    let mut notes: Vec<String> = vec![];

    let repaired = cleanup_tmp_files(project_path, &vc_root)?;
    out.repaired_tmp_files = repaired;
    if repaired > 0 {
        notes.push(format!("cleaned_tmp_files={repaired}"));
    }

    let truncated = truncate_wal_if_corrupted(&vc_root.join(WAL_FILE))?;
    out.truncated_wal_bytes = truncated;
    if truncated > 0 {
        notes.push(format!("truncated_wal_bytes={truncated}"));
    }

    let rebuilt = rebuild_head_from_wal(project_path, &vc_root)?;
    out.rebuilt_head_entities = rebuilt;

    let appended = rebuild_call_index_from_wal(&vc_root)?;
    out.appended_call_index = appended;

    let event_seq = next_event_seq(&vc_root, project_path)?;
    append_wal(
        &vc_root.join(WAL_FILE),
        &WalRecord {
            event_seq,
            ts: chrono::Utc::now().timestamp_millis(),
            r#type: WalType::RecoveryRepair,
            tx_id: format!("tx_{}", uuid::Uuid::new_v4()),
            call_id: None,
            actor: Some(VcActor::System),
            entity_id: RECOVERY_ENTITY_ID.to_string(),
            from_revision: None,
            to_revision: None,
            expected_revision: None,
            before_hash: None,
            after_hash: None,
            patch_hash: None,
            status: WalStatus::Ok,
            detail: Some(if notes.is_empty() {
                "recover_checked:no_changes".to_string()
            } else {
                notes.join(";")
            }),
        },
    )?;

    out.notes = notes;
    out.ok = true;
    Ok(out)
}

#[cfg(test)]
pub fn current_lock_count(project_path: &str) -> Result<usize, AppError> {
    let vc_root = vc_root_dir(project_path);
    let locks_dir = vc_root.join("locks");
    if !locks_dir.exists() {
        return Ok(0);
    }

    let mut count = 0usize;
    for entry in std::fs::read_dir(locks_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            count += 1;
        }
    }
    Ok(count)
}
