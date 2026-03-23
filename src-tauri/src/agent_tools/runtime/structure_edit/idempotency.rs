use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::AppError;

use super::{StructureEditArgs, StructureNodeType, StructureOp, TOOL_NAME};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IdempotencyRecord {
    pub(super) key: String,
    pub(super) fingerprint: String,
    pub(super) data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tx_id: Option<String>,
    pub(super) created_at_ms: i64,
}

pub(super) fn fingerprint(args: &StructureEditArgs) -> String {
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        op: StructureOp,
        node_type: StructureNodeType,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_ref: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_ref: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        position: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
    }

    let fp = Fingerprint {
        op: args.op,
        node_type: args.node_type,
        target_ref: args.target_ref.as_deref(),
        parent_ref: args.parent_ref.as_deref(),
        position: args.position,
        title: args.title.as_deref(),
    };

    serde_json::to_string(&fp).unwrap_or_default()
}

fn idempotency_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path)
        .join(".magic_nover")
        .join("tool_idempotency")
        .join(TOOL_NAME)
}

fn safe_key(key: &str) -> String {
    let mut out = String::new();
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 80 {
            break;
        }
    }
    if out.is_empty() {
        "key".to_string()
    } else {
        out
    }
}

fn idempotency_path(project_path: &str, key: &str) -> PathBuf {
    idempotency_dir(project_path).join(format!("{}.json", safe_key(key)))
}

pub(super) fn load_idempotency_record(project_path: &str, key: &str) -> Option<IdempotencyRecord> {
    let path = idempotency_path(project_path, key);
    if !path.exists() {
        return None;
    }
    crate::services::read_json(&path).ok()
}

pub(super) fn save_idempotency_record(
    project_path: &str,
    key: &str,
    record: &IdempotencyRecord,
) -> Result<(), AppError> {
    let dir = idempotency_dir(project_path);
    crate::services::ensure_dir(&dir)?;
    let path = idempotency_path(project_path, key);
    crate::utils::atomic_write::atomic_write_json(&path, record)
}
