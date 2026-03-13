use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::models::{AppError, ErrorCode};
use crate::services::{ensure_dir, read_json};
use crate::utils::atomic_write::atomic_write_json;

use super::{session_card_path, sessions_root};

pub const SESSION_CARD_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonUpdatesProposedEntry {
    pub bundle_id: String,
    pub delta_id: String,
    pub scope_ref: String,
    #[serde(default)]
    pub kinds: Vec<String>,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonUpdatesAcceptedEntry {
    pub delta_id: String,
    pub applied_at: i64,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolled_back_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentSessionCardV1 {
    pub schema_version: i32,
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub canon_updates_proposed: Vec<CanonUpdatesProposedEntry>,
    #[serde(default)]
    pub canon_updates_accepted: Vec<CanonUpdatesAcceptedEntry>,
    #[serde(default)]
    pub knowledge_refs_written: Vec<String>,
    #[serde(default)]
    pub updated_at: i64,
}

fn is_safe_session_id_for_path(session_id: &str) -> bool {
    let s = session_id.trim();
    if s.is_empty() {
        return false;
    }

    // Conservative Windows-safe check.
    !s.chars().any(|ch| {
        matches!(
            ch,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r'
        )
    })
}

fn merge_unique(mut base: Vec<String>, extra: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::<String>::new();
    for v in base.iter() {
        let t = v.trim();
        if !t.is_empty() {
            seen.insert(t.to_string());
        }
    }

    for v in extra {
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        if seen.insert(t.to_string()) {
            base.push(t.to_string());
        }
    }

    base
}

pub fn read_session_card(
    project_path: &Path,
    session_id: &str,
) -> Result<Option<AgentSessionCardV1>, AppError> {
    if !is_safe_session_id_for_path(session_id) {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "invalid session_id for session card".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_CARD_INVALID_SESSION_ID",
                "session_id": session_id,
            })),
            recoverable: Some(true),
        });
    }

    let path = session_card_path(project_path, session_id);
    if !path.exists() {
        return Ok(None);
    }

    let card: AgentSessionCardV1 = read_json(&path).map_err(|err| AppError {
        code: ErrorCode::JsonParseError,
        message: format!("failed to parse session card: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_CARD_PARSE_FAILED",
            "path": path.to_string_lossy(),
            "session_id": session_id,
        })),
        recoverable: Some(true),
    })?;

    Ok(Some(card))
}

pub fn write_session_card(project_path: &Path, card: &AgentSessionCardV1) -> Result<(), AppError> {
    if !is_safe_session_id_for_path(&card.session_id) {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "invalid session_id for session card".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_CARD_INVALID_SESSION_ID",
                "session_id": card.session_id,
            })),
            recoverable: Some(true),
        });
    }

    let root = sessions_root(project_path);
    ensure_dir(&root)?;
    let path = session_card_path(project_path, &card.session_id);
    atomic_write_json(&path, card).map_err(|err| AppError {
        code: ErrorCode::IoError,
        message: format!("failed to write session card: {err}"),
        details: Some(serde_json::json!({
            "code": "E_AGENT_SESSION_CARD_WRITE_FAILED",
            "path": path.to_string_lossy(),
            "session_id": card.session_id,
        })),
        recoverable: Some(true),
    })
}

pub fn record_canon_updates_proposed(
    project_path: &Path,
    session_id: &str,
    branch_id: Option<&String>,
    entry: CanonUpdatesProposedEntry,
) -> Result<(), AppError> {
    if !is_safe_session_id_for_path(session_id) {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "invalid session_id for session card".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_CARD_INVALID_SESSION_ID",
                "session_id": session_id,
            })),
            recoverable: Some(true),
        });
    }

    let now = chrono::Utc::now().timestamp_millis();
    let mut card =
        read_session_card(project_path, session_id)?.unwrap_or_else(|| AgentSessionCardV1 {
            schema_version: SESSION_CARD_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            branch_id: None,
            canon_updates_proposed: Vec::new(),
            canon_updates_accepted: Vec::new(),
            knowledge_refs_written: Vec::new(),
            updated_at: now,
        });

    card.schema_version = SESSION_CARD_SCHEMA_VERSION;
    card.session_id = session_id.to_string();
    if let Some(b) = branch_id.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }) {
        card.branch_id = Some(b);
    }

    if let Some(existing) = card
        .canon_updates_proposed
        .iter_mut()
        .find(|it| it.delta_id == entry.delta_id)
    {
        *existing = entry;
    } else {
        card.canon_updates_proposed.push(entry);
    }

    card.updated_at = now;
    write_session_card(project_path, &card)
}

pub fn record_canon_updates_accepted(
    project_path: &Path,
    session_id: &str,
    branch_id: Option<&String>,
    entry: CanonUpdatesAcceptedEntry,
    knowledge_refs_written: &[String],
) -> Result<(), AppError> {
    if !is_safe_session_id_for_path(session_id) {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "invalid session_id for session card".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_CARD_INVALID_SESSION_ID",
                "session_id": session_id,
            })),
            recoverable: Some(true),
        });
    }

    let now = chrono::Utc::now().timestamp_millis();
    let mut card =
        read_session_card(project_path, session_id)?.unwrap_or_else(|| AgentSessionCardV1 {
            schema_version: SESSION_CARD_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            branch_id: None,
            canon_updates_proposed: Vec::new(),
            canon_updates_accepted: Vec::new(),
            knowledge_refs_written: Vec::new(),
            updated_at: now,
        });

    card.schema_version = SESSION_CARD_SCHEMA_VERSION;
    card.session_id = session_id.to_string();
    if let Some(b) = branch_id.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }) {
        card.branch_id = Some(b);
    }

    if let Some(existing) = card
        .canon_updates_accepted
        .iter_mut()
        .find(|it| it.delta_id == entry.delta_id)
    {
        *existing = entry;
    } else {
        card.canon_updates_accepted.push(entry);
    }

    card.knowledge_refs_written = merge_unique(card.knowledge_refs_written, knowledge_refs_written);
    card.updated_at = now;
    write_session_card(project_path, &card)
}

pub fn record_canon_updates_rolled_back(
    project_path: &Path,
    session_id: &str,
    branch_id: Option<&String>,
    delta_id: &str,
    rollback_token: Option<&str>,
    rolled_back_at: i64,
) -> Result<(), AppError> {
    if !is_safe_session_id_for_path(session_id) {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "invalid session_id for session card".to_string(),
            details: Some(serde_json::json!({
                "code": "E_AGENT_SESSION_CARD_INVALID_SESSION_ID",
                "session_id": session_id,
            })),
            recoverable: Some(true),
        });
    }

    let now = chrono::Utc::now().timestamp_millis();
    let mut card =
        read_session_card(project_path, session_id)?.unwrap_or_else(|| AgentSessionCardV1 {
            schema_version: SESSION_CARD_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            branch_id: None,
            canon_updates_proposed: Vec::new(),
            canon_updates_accepted: Vec::new(),
            knowledge_refs_written: Vec::new(),
            updated_at: now,
        });

    card.schema_version = SESSION_CARD_SCHEMA_VERSION;
    card.session_id = session_id.to_string();
    if let Some(b) = branch_id.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }) {
        card.branch_id = Some(b);
    }

    if let Some(existing) = card
        .canon_updates_accepted
        .iter_mut()
        .find(|it| it.delta_id == delta_id)
    {
        existing.rolled_back_at = Some(rolled_back_at);
        if existing.rollback_token.is_none() {
            existing.rollback_token = rollback_token
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
        }
    }

    card.updated_at = now;
    write_session_card(project_path, &card)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("magic_session_card_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn record_roundtrip_proposed_and_accepted() {
        let project = temp_project_dir();
        let session_id = "worker_test_1";

        record_canon_updates_proposed(
            &project,
            session_id,
            Some(&"branch/main".to_string()),
            CanonUpdatesProposedEntry {
                bundle_id: "kb1".to_string(),
                delta_id: "kd1".to_string(),
                scope_ref: "chapter:x".to_string(),
                kinds: vec!["chapter_summary".to_string()],
                ts: 123,
            },
        )
        .unwrap();

        record_canon_updates_accepted(
            &project,
            session_id,
            Some(&"branch/main".to_string()),
            CanonUpdatesAcceptedEntry {
                delta_id: "kd1".to_string(),
                applied_at: 456,
                targets: vec!["chapter_summaries/x.json".to_string()],
                rollback_token: Some("rbk".to_string()),
                rolled_back_at: None,
            },
            &["chapter_summaries/x.json".to_string()],
        )
        .unwrap();

        record_canon_updates_rolled_back(
            &project,
            session_id,
            Some(&"branch/main".to_string()),
            "kd1",
            Some("rbk"),
            789,
        )
        .unwrap();

        let card = read_session_card(&project, session_id).unwrap().unwrap();
        assert_eq!(card.session_id, session_id);
        assert_eq!(card.branch_id.as_deref(), Some("branch/main"));
        assert_eq!(card.canon_updates_proposed.len(), 1);
        assert_eq!(card.canon_updates_accepted.len(), 1);
        assert_eq!(card.canon_updates_accepted[0].rolled_back_at, Some(789));
        assert!(card
            .knowledge_refs_written
            .iter()
            .any(|v| v == "chapter_summaries/x.json"));
    }
}
