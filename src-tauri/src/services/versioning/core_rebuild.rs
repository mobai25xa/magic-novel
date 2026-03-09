use std::fs;
use std::path::{Path, PathBuf};

use crate::models::AppError;
use crate::services::versioning_port::EntityHead;

use super::core_call_index::{append_call_index, find_call_index_by_call_id};
use super::core_head_wal::{read_wal_lines, write_head_state_atomic};
use super::core_layout::{
    entity_path_from_entity_id, CALL_INDEX_FILE, ENTITY_TX_TMP_SUFFIX, HEAD_FILE, LOCKS_DIR,
    WAL_FILE,
};
use super::core_types::{CallIndexRecord, HeadState, WalRecord, WalStatus, WalType};
use super::core_utils::{app_err_vc, compute_json_hash};

pub(crate) fn cleanup_tmp_files(project_path: &str, vc_root: &Path) -> Result<i64, AppError> {
    let mut count = 0;

    let manuscripts = PathBuf::from(project_path).join("manuscripts");
    if manuscripts.exists() {
        cleanup_tmp_in_dir(&manuscripts, &mut count)?;
    }

    let locks_dir = vc_root.join(LOCKS_DIR);
    if locks_dir.exists() {
        for entry in fs::read_dir(&locks_dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_file() {
                let _ = fs::remove_file(path);
                count += 1;
            }
        }
    }

    Ok(count)
}

fn cleanup_tmp_in_dir(dir: &Path, count: &mut i64) -> Result<(), AppError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            cleanup_tmp_in_dir(&path, count)?;
            continue;
        }

        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if name.contains(ENTITY_TX_TMP_SUFFIX) {
                let _ = fs::remove_file(&path);
                *count += 1;
            }
        }
    }
    Ok(())
}

pub(crate) fn rebuild_head_from_wal(project_path: &str, vc_root: &Path) -> Result<i64, AppError> {
    let wal_path = vc_root.join(WAL_FILE);
    let lines = read_wal_lines(&wal_path)?;

    let mut entities: std::collections::HashMap<String, EntityHead> =
        std::collections::HashMap::new();
    let mut last_seq = 0;

    for line in lines {
        if let Ok(rec) = serde_json::from_str::<WalRecord>(&line) {
            if rec.event_seq > last_seq {
                last_seq = rec.event_seq;
            }
            if rec.r#type == WalType::Commit && rec.status == WalStatus::Ok {
                let ent = entities.entry(rec.entity_id.clone()).or_default();
                if let Some(to_rev) = rec.to_revision {
                    ent.revision = to_rev;
                }
                if let Some(h) = &rec.after_hash {
                    ent.json_hash = h.clone();
                }
                ent.last_call_id = rec.call_id.clone();
                ent.last_tx_id = Some(rec.tx_id.clone());
                ent.updated_at = rec.ts;
            }
        }
    }

    for (entity_id, head) in entities.iter_mut() {
        let entity_path = match entity_path_from_entity_id(project_path, entity_id) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !entity_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&entity_path)?;
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            app_err_vc(
                "E_VC_IO_WRITE_FAIL",
                format!("failed to parse entity json during recover: {e}"),
                true,
            )
        })?;
        head.json_hash = compute_json_hash(&json);
        head.updated_at = chrono::Utc::now().timestamp_millis();
    }

    let st = HeadState {
        project_path: project_path.to_string(),
        last_event_seq: last_seq,
        entities,
    };

    write_head_state_atomic(&vc_root.join(HEAD_FILE), &st)?;

    Ok(st.entities.len() as i64)
}

pub(crate) fn rebuild_call_index_from_wal(vc_root: &Path) -> Result<i64, AppError> {
    let wal_path = vc_root.join(WAL_FILE);
    if !wal_path.exists() {
        return Ok(0);
    }

    let lines = read_wal_lines(&wal_path)?;
    let call_index_path = vc_root.join(CALL_INDEX_FILE);

    let mut appended = 0;

    for line in lines {
        let Ok(rec) = serde_json::from_str::<WalRecord>(&line) else {
            continue;
        };

        if rec.r#type != WalType::Commit || rec.status != WalStatus::Ok {
            continue;
        }

        let Some(call_id) = rec.call_id.clone() else {
            continue;
        };

        if find_call_index_by_call_id(&call_index_path, &call_id)?.is_some() {
            continue;
        }

        let from_rev = rec.from_revision.unwrap_or(0);
        let to_rev = rec.to_revision.unwrap_or(from_rev);

        append_call_index(
            &call_index_path,
            &CallIndexRecord {
                call_id,
                entity_id: rec.entity_id,
                tx_id: rec.tx_id,
                from_revision: from_rev,
                to_revision: to_rev,
                event_seq: rec.event_seq,
                ts: rec.ts,
            },
        )?;
        appended += 1;
    }

    Ok(appended)
}
