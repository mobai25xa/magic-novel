use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent_tools::tools::r#ref::{
    normalize_project_relative_path, parse_tool_ref, RefError, RefKind, ToolRef,
};
use crate::knowledge::types::{
    KnowledgeAcceptPolicy, KnowledgeOp, KnowledgeProposalBundle, KnowledgeProposalItem,
};
use crate::models::AppError;
use crate::services;
use crate::utils::atomic_write::atomic_write_json;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeWriteOp {
    Propose,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeWriteChangeKind {
    Add,
    Update,
    Delete,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeWriteChange {
    pub target_ref: String,
    pub kind: KnowledgeWriteChangeKind,
    pub fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeWriteArgs {
    pub op: KnowledgeWriteOp,
    pub changes: Vec<KnowledgeWriteChange>,
    pub evidence_refs: Option<Vec<String>>,
    pub dry_run: Option<bool>,
    pub idempotency_key: Option<String>,
    #[serde(default, rename = "timeout_ms")]
    pub _timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeWriteOutput {
    pub delta_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeWriteRun {
    pub output: KnowledgeWriteOutput,
    pub read_set: Option<Vec<String>>,
    pub write_set: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeWriteError {
    pub code: &'static str,
    pub message: String,
}

pub(crate) fn validate_knowledge_write_input_shape(
    input: &serde_json::Value,
) -> Result<(), KnowledgeWriteError> {
    let Some(changes) = input.get("changes") else {
        return Ok(());
    };

    let Some(change_items) = changes.as_array() else {
        return Err(type_mismatch_error("changes", "array", changes));
    };

    for (idx, change) in change_items.iter().enumerate() {
        let Some(change_obj) = change.as_object() else {
            return Err(type_mismatch_error(
                &format!("changes[{idx}]"),
                "object",
                change,
            ));
        };

        let path = format!("changes[{idx}].fields");
        let Some(fields) = change_obj.get("fields") else {
            return Err(KnowledgeWriteError {
                code: "E_TOOL_SCHEMA_INVALID",
                message: format!("{path} is required"),
            });
        };

        if !fields.is_object() {
            return Err(type_mismatch_error(&path, "object", fields));
        }
    }

    Ok(())
}

pub fn run_knowledge_write(
    project_path: &str,
    call_id: &str,
    args: KnowledgeWriteArgs,
) -> Result<KnowledgeWriteRun, KnowledgeWriteError> {
    let project_path = project_path.trim();
    if project_path.is_empty() {
        return Err(KnowledgeWriteError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "missing project_path".to_string(),
        });
    }

    if args.changes.is_empty() {
        return Err(KnowledgeWriteError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "changes must be a non-empty array".to_string(),
        });
    }

    let dry_run = args.dry_run.unwrap_or(false);

    let idempotency_key = args
        .idempotency_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if !dry_run {
        let Some(key) = idempotency_key else {
            return Err(KnowledgeWriteError {
                code: "E_TOOL_SCHEMA_INVALID",
                message: "idempotency_key is required when dry_run=false".to_string(),
            });
        };
        if key.len() > MAX_IDEMPOTENCY_KEY_LEN {
            return Err(KnowledgeWriteError {
                code: "E_TOOL_SCHEMA_INVALID",
                message: "idempotency_key must be 1..=128 characters".to_string(),
            });
        }
    }

    validate_payload_budget(&args)?;

    // Idempotency fast-path (commit mode only).
    if !dry_run {
        if let Some(key) = idempotency_key {
            if let Some(record) = load_idempotency_record(project_path, key) {
                if record.fingerprint != fingerprint_request(&args) {
                    return Err(KnowledgeWriteError {
                        code: "E_TOOL_SCHEMA_INVALID",
                        message: "idempotency_key was already used for a different request"
                            .to_string(),
                    });
                }

                let output: KnowledgeWriteOutput =
                    serde_json::from_value(record.data).map_err(|_| KnowledgeWriteError {
                        code: "E_INTERNAL",
                        message: "failed to load idempotent output".to_string(),
                    })?;

                let (read_set, write_set) =
                    build_meta_sets(project_path, &args, Some(&record.artifact_ref));
                return Ok(KnowledgeWriteRun {
                    output,
                    read_set,
                    write_set,
                });
            }
        }
    }

    // Build bundle -> gate delta.
    let branch_id = resolve_active_branch_id(Path::new(project_path));
    let now = chrono::Utc::now().timestamp_millis();
    let bundle_id = format!("kbundle_{}", uuid::Uuid::new_v4());
    let scope_ref = "scope_ref:knowledge_write".to_string();

    let evidence_refs = args
        .evidence_refs
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(50)
        .collect::<Vec<_>>();

    let mut proposal_items = Vec::new();
    for (idx, change) in args.changes.iter().enumerate() {
        let (target_internal_ref, kind) = normalize_target_ref_and_kind(&change.target_ref)?;
        let op = match change.kind {
            KnowledgeWriteChangeKind::Add => KnowledgeOp::Create,
            KnowledgeWriteChangeKind::Update => KnowledgeOp::Update,
            KnowledgeWriteChangeKind::Delete => KnowledgeOp::Archive,
        };

        let target_revision = match op {
            KnowledgeOp::Update | KnowledgeOp::Archive | KnowledgeOp::Restore => {
                load_existing_revision(Path::new(project_path), &target_internal_ref)
            }
            KnowledgeOp::Create => None,
        };

        proposal_items.push(KnowledgeProposalItem {
            item_id: format!("kitem_{}", uuid::Uuid::new_v4()),
            kind,
            op,
            target_ref: Some(target_internal_ref),
            target_revision,
            fields: serde_json::Value::Object(change.fields.clone()),
            evidence_refs: evidence_refs.clone(),
            source_refs: vec![format!("tool_call_id:{call_id}")],
            change_reason: format!("knowledge_write propose #{idx}"),
            accept_policy: KnowledgeAcceptPolicy::Manual,
        });
    }

    let bundle = KnowledgeProposalBundle {
        schema_version: crate::knowledge::types::KNOWLEDGE_SCHEMA_VERSION,
        bundle_id,
        scope_ref,
        branch_id: Some(branch_id),
        source_session_id: call_id.to_string(),
        source_review_id: None,
        generated_at: now,
        proposal_items,
    };

    let mut delta =
        crate::knowledge::writeback::gate_bundle(Path::new(project_path), &bundle, None)
            .map_err(map_app_error)?;

    // Normalize delta_id if idempotency_key exists (helps diffing in logs).
    if let Some(key) = idempotency_key {
        delta.knowledge_delta_id = format!("kdelta_{}", safe_key(key));
    }

    let conflicts = (!delta.conflicts.is_empty())
        .then(|| {
            serde_json::to_value(&delta.conflicts)
                .unwrap_or_else(|_| serde_json::Value::Array(vec![]))
        })
        .and_then(|v| v.as_array().cloned());

    let artifact_ref = build_artifact_ref(&delta.knowledge_delta_id);
    if !dry_run {
        persist_artifact(
            project_path,
            &artifact_ref,
            &bundle,
            &delta,
            call_id,
            idempotency_key,
            &args,
        )?;
        if let Some(key) = idempotency_key {
            let record = IdempotencyRecord {
                key: key.to_string(),
                fingerprint: fingerprint_request(&args),
                data: serde_json::to_value(KnowledgeWriteOutput {
                    delta_id: delta.knowledge_delta_id.clone(),
                    status: "proposed".to_string(),
                    conflicts: conflicts.clone(),
                    next_action: delta
                        .conflicts
                        .is_empty()
                        .then_some("none".to_string())
                        .or_else(|| Some("askuser".to_string())),
                })
                .unwrap_or(serde_json::json!({})),
                artifact_ref: artifact_ref.clone(),
                created_at_ms: now,
            };
            save_idempotency_record(project_path, key, &record).map_err(map_app_error)?;
        }
    }

    let output = KnowledgeWriteOutput {
        delta_id: delta.knowledge_delta_id.clone(),
        status: "proposed".to_string(),
        conflicts,
        next_action: delta
            .conflicts
            .is_empty()
            .then_some("none".to_string())
            .or_else(|| Some("askuser".to_string())),
    };

    let (read_set, write_set) = build_meta_sets(
        project_path,
        &args,
        (!dry_run).then_some(artifact_ref.as_str()),
    );

    Ok(KnowledgeWriteRun {
        output,
        read_set,
        write_set,
    })
}

const TOOL_NAME: &str = "knowledge_write";
const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;
const MAX_FIELDS_JSON_CHARS: usize = 50_000;
const MAX_TOTAL_FIELDS_JSON_CHARS: usize = 150_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdempotencyRecord {
    key: String,
    fingerprint: String,
    data: serde_json::Value,
    artifact_ref: String,
    created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchStateDoc {
    schema_version: i32,
    active_branch_id: String,
    updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnowledgeWriteProposalArtifact {
    schema_version: i32,
    tool: String,
    created_at: i64,
    call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    idempotency_key: Option<String>,
    request_fingerprint: String,
    bundle: KnowledgeProposalBundle,
    delta: crate::knowledge::types::KnowledgeDelta,
}

fn map_app_error(err: AppError) -> KnowledgeWriteError {
    let (code, message) = match err.code {
        crate::models::ErrorCode::InvalidArgument => ("E_TOOL_SCHEMA_INVALID", err.message),
        crate::models::ErrorCode::IoError => ("E_IO", err.message),
        crate::models::ErrorCode::Conflict => ("E_CONFLICT", err.message),
        _ => ("E_INTERNAL", err.message),
    };
    KnowledgeWriteError { code, message }
}

fn resolve_active_branch_id(project_path: &Path) -> String {
    let root = services::knowledge_paths::resolve_knowledge_root_for_read(project_path);
    let p = root.join("branch_state.json");
    if let Ok(raw) = std::fs::read_to_string(&p) {
        if let Ok(doc) = serde_json::from_str::<BranchStateDoc>(&raw) {
            let id = doc.active_branch_id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }
    "branch/main".to_string()
}

fn normalize_target_ref_and_kind(
    target_ref: &str,
) -> Result<(String, String), KnowledgeWriteError> {
    let virtual_path = parse_knowledge_target_ref(target_ref)?;
    let rel = virtual_path.trim_start_matches(".magic_novel/").trim();
    if rel.is_empty() {
        return Err(KnowledgeWriteError {
            code: "E_REF_INVALID",
            message: "target_ref must point to a knowledge item path".to_string(),
        });
    }
    if !rel.to_lowercase().ends_with(".json") {
        return Err(KnowledgeWriteError {
            code: "E_REF_INVALID",
            message: "target_ref must point to a .json knowledge object".to_string(),
        });
    }

    let kind = infer_kind_from_rel(rel).ok_or_else(|| KnowledgeWriteError {
        code: "E_REF_INVALID",
        message: "target_ref kind is not supported in v0".to_string(),
    })?;

    Ok((rel.to_string(), kind.to_string()))
}

fn infer_kind_from_rel(rel: &str) -> Option<&'static str> {
    let first = rel.split('/').next()?.trim();
    match first {
        "characters" => Some("character"),
        "locations" => Some("location"),
        "organizations" => Some("organization"),
        "rules" => Some("rule"),
        "terms" => Some("term"),
        "plotlines" => Some("plotline"),
        "style_rules" => Some("style_rule"),
        "sources" => Some("source"),
        "chapter_summaries" => Some("chapter_summary"),
        "recent_facts" => Some("recent_fact"),
        "foreshadow" => Some("foreshadow"),
        _ => None,
    }
}

fn load_existing_revision(project_path: &Path, target_internal_ref: &str) -> Option<i64> {
    let p = services::knowledge_paths::resolve_knowledge_physical_path(
        project_path,
        &format!(".magic_novel/{target_internal_ref}"),
    );
    let raw = std::fs::read_to_string(&p).ok()?;
    let v = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    v.get("revision").and_then(|x| x.as_i64())
}

fn validate_payload_budget(args: &KnowledgeWriteArgs) -> Result<(), KnowledgeWriteError> {
    let mut total = 0_usize;
    for (idx, ch) in args.changes.iter().enumerate() {
        let s = serde_json::to_string(&ch.fields).unwrap_or_default();
        let len = s.chars().count();
        if len > MAX_FIELDS_JSON_CHARS {
            return Err(KnowledgeWriteError {
                code: "E_PAYLOAD_TOO_LARGE",
                message: format!(
                    "changes[{idx}].fields payload is too large: {len} chars (max {MAX_FIELDS_JSON_CHARS})"
                ),
            });
        }
        total = total.saturating_add(len);
    }

    if total > MAX_TOTAL_FIELDS_JSON_CHARS {
        return Err(KnowledgeWriteError {
            code: "E_PAYLOAD_TOO_LARGE",
            message: format!(
                "total changes[*].fields payload is too large: {total} chars (max {MAX_TOTAL_FIELDS_JSON_CHARS})"
            ),
        });
    }

    Ok(())
}

fn fingerprint_request(args: &KnowledgeWriteArgs) -> String {
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        op: KnowledgeWriteOp,
        changes: &'a Vec<KnowledgeWriteChange>,
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence_refs: Option<&'a Vec<String>>,
    }

    let fp = Fingerprint {
        op: args.op,
        changes: &args.changes,
        evidence_refs: args.evidence_refs.as_ref(),
    };

    serde_json::to_string(&fp).unwrap_or_default()
}

fn tool_state_dir(project_path: &str) -> PathBuf {
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
    tool_state_dir(project_path).join(format!("{}.json", safe_key(key)))
}

fn load_idempotency_record(project_path: &str, key: &str) -> Option<IdempotencyRecord> {
    let path = idempotency_path(project_path, key);
    if !path.exists() {
        return None;
    }
    crate::services::read_json(&path).ok()
}

fn save_idempotency_record(
    project_path: &str,
    key: &str,
    record: &IdempotencyRecord,
) -> Result<(), AppError> {
    let dir = tool_state_dir(project_path);
    crate::services::ensure_dir(&dir)?;
    let path = idempotency_path(project_path, key);
    crate::utils::atomic_write::atomic_write_json(&path, record)
}

fn build_artifact_ref(delta_id: &str) -> String {
    format!("artifact:magic_novel/tools/knowledge_write/proposals/{delta_id}.json")
}

fn artifact_path(project_path: &str, artifact_ref: &str) -> Result<PathBuf, KnowledgeWriteError> {
    let (kind, path) = artifact_ref
        .split_once(':')
        .ok_or_else(|| KnowledgeWriteError {
            code: "E_REF_INVALID",
            message: "artifact_ref is invalid".to_string(),
        })?;
    if kind != "artifact" {
        return Err(KnowledgeWriteError {
            code: "E_REF_INVALID",
            message: "artifact_ref must be an artifact ref".to_string(),
        });
    }
    let normalized =
        normalize_project_relative_path(path, false).map_err(|err| KnowledgeWriteError {
            code: err.code,
            message: err.message,
        })?;
    Ok(PathBuf::from(project_path).join(normalized))
}

fn persist_artifact(
    project_path: &str,
    artifact_ref: &str,
    bundle: &KnowledgeProposalBundle,
    delta: &crate::knowledge::types::KnowledgeDelta,
    call_id: &str,
    idempotency_key: Option<&str>,
    args: &KnowledgeWriteArgs,
) -> Result<(), KnowledgeWriteError> {
    let path = artifact_path(project_path, artifact_ref)?;
    if let Some(parent) = path.parent() {
        crate::services::ensure_dir(parent).map_err(map_app_error)?;
    }

    let artifact = KnowledgeWriteProposalArtifact {
        schema_version: 1,
        tool: TOOL_NAME.to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
        call_id: call_id.to_string(),
        idempotency_key: idempotency_key.map(|s| s.to_string()),
        request_fingerprint: fingerprint_request(args),
        bundle: bundle.clone(),
        delta: delta.clone(),
    };

    atomic_write_json(&path, &artifact).map_err(map_app_error)
}

fn build_meta_sets(
    _project_path: &str,
    args: &KnowledgeWriteArgs,
    artifact_ref: Option<&str>,
) -> (Option<Vec<String>>, Option<Vec<String>>) {
    let mut read_set = Vec::new();
    if let Some(evidence) = args.evidence_refs.as_ref() {
        read_set.extend(evidence.iter().map(|item| canonicalize_meta_ref(item)));
    }
    for ch in &args.changes {
        read_set.push(canonicalize_meta_ref(&ch.target_ref));
    }

    let write_set = artifact_ref.map(|ar| vec![ar.to_string()]);
    (Some(read_set), write_set)
}

fn parse_knowledge_target_ref(raw: &str) -> Result<String, KnowledgeWriteError> {
    if services::knowledge_paths::looks_like_knowledge_input(raw) {
        return services::knowledge_paths::normalize_knowledge_virtual_path(raw)
            .map_err(map_tool_ref_error);
    }

    match parse_tool_ref(raw) {
        Ok(tref) => {
            if tref.kind != RefKind::Knowledge {
                return Err(KnowledgeWriteError {
                    code: "E_REF_KIND_UNSUPPORTED",
                    message: "target_ref must be a knowledge ref".to_string(),
                });
            }

            services::knowledge_paths::normalize_knowledge_virtual_path(&tref.path)
                .map_err(map_tool_ref_error)
        }
        Err(parse_err) => Err(map_tool_ref_error(parse_err)),
    }
}

fn canonicalize_meta_ref(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if services::knowledge_paths::looks_like_knowledge_input(trimmed) {
        if let Ok(path) = services::knowledge_paths::normalize_knowledge_virtual_path(trimmed) {
            return format!("knowledge:{path}");
        }
    }

    if let Ok(tref) = parse_tool_ref(trimmed) {
        return canonicalize_parsed_ref(&tref);
    }

    trimmed.to_string()
}

fn canonicalize_parsed_ref(tref: &ToolRef) -> String {
    match tref.kind {
        RefKind::Book => format!("book:{}", tref.path),
        RefKind::Volume => format!("volume:{}", tref.path),
        RefKind::Chapter => format!("chapter:{}", tref.path),
        RefKind::Artifact => format!("artifact:{}", tref.path),
        RefKind::Knowledge => {
            let path = services::knowledge_paths::normalize_knowledge_virtual_path(&tref.path)
                .unwrap_or_else(|_| tref.path.clone());
            format!("knowledge:{path}")
        }
    }
}

fn map_tool_ref_error(err: RefError) -> KnowledgeWriteError {
    KnowledgeWriteError {
        code: err.code,
        message: err.message,
    }
}

fn type_mismatch_error(
    path: &str,
    expected: &str,
    actual: &serde_json::Value,
) -> KnowledgeWriteError {
    KnowledgeWriteError {
        code: "E_TOOL_SCHEMA_INVALID",
        message: format!(
            "{path} must be an {expected}, got {}",
            json_type_name(actual)
        ),
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_project() -> (tempfile::TempDir, String) {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().to_string_lossy().to_string();
        std::fs::create_dir_all(Path::new(&project_path).join(".magic_novel")).expect("dir");
        (dir, project_path)
    }

    fn object_fields(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        value.as_object().cloned().expect("object fields")
    }

    #[test]
    fn invalid_target_ref_is_rejected() {
        let (_dir, project_path) = setup_project();
        let err = run_knowledge_write(
            &project_path,
            "call_1",
            KnowledgeWriteArgs {
                op: KnowledgeWriteOp::Propose,
                changes: vec![KnowledgeWriteChange {
                    target_ref: "chapter:manuscripts/vol_1/ch_1.json".to_string(),
                    kind: KnowledgeWriteChangeKind::Add,
                    fields: object_fields(serde_json::json!({"summary": "x"})),
                }],
                evidence_refs: None,
                dry_run: Some(true),
                idempotency_key: None,
                _timeout_ms: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "E_REF_KIND_UNSUPPORTED");
    }

    #[test]
    fn idempotency_key_reuses_delta_id() {
        let (_dir, project_path) = setup_project();

        let args = KnowledgeWriteArgs {
            op: KnowledgeWriteOp::Propose,
            changes: vec![KnowledgeWriteChange {
                target_ref: "knowledge:.magic_novel/terms/foo.json".to_string(),
                kind: KnowledgeWriteChangeKind::Add,
                fields: object_fields(serde_json::json!({"summary": "x"})),
            }],
            evidence_refs: None,
            dry_run: Some(false),
            idempotency_key: Some("same-key".to_string()),
            _timeout_ms: None,
        };

        let first = run_knowledge_write(&project_path, "call_1", args.clone()).expect("first");
        let second = run_knowledge_write(&project_path, "call_2", args.clone()).expect("second");

        assert_eq!(first.output.delta_id, second.output.delta_id);

        let artifact_ref = build_artifact_ref(&first.output.delta_id);
        let artifact_path = artifact_path(&project_path, &artifact_ref).expect("path");
        assert!(artifact_path.exists());
    }

    #[test]
    fn shorthand_target_ref_is_canonicalized_in_read_set() {
        let (_dir, project_path) = setup_project();

        let run = run_knowledge_write(
            &project_path,
            "call_1",
            KnowledgeWriteArgs {
                op: KnowledgeWriteOp::Propose,
                changes: vec![KnowledgeWriteChange {
                    target_ref: "terms/foo.json".to_string(),
                    kind: KnowledgeWriteChangeKind::Add,
                    fields: object_fields(serde_json::json!({"summary": "x"})),
                }],
                evidence_refs: Some(vec!["knowledge:.magic_novel/terms/foo.json".to_string()]),
                dry_run: Some(true),
                idempotency_key: None,
                _timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(
            run.read_set.as_ref().unwrap_or(&Vec::new()),
            &vec![
                "knowledge:.magic_novel/terms/foo.json".to_string(),
                "knowledge:.magic_novel/terms/foo.json".to_string()
            ]
        );
    }

    #[test]
    fn knowledge_write_args_deserialization_rejects_non_object_fields() {
        let err = serde_json::from_value::<KnowledgeWriteArgs>(serde_json::json!({
            "op": "propose",
            "changes": [
                {
                    "target_ref": "knowledge:.magic_novel/terms/foo.json",
                    "kind": "add",
                    "fields": "summary = foo"
                }
            ],
            "dry_run": true
        }))
        .expect_err("non-object fields should fail deserialization");

        assert!(err.to_string().contains("expected a map"));
    }

    #[test]
    fn validate_input_shape_reports_fields_path() {
        let err = validate_knowledge_write_input_shape(&serde_json::json!({
            "op": "propose",
            "changes": [
                {
                    "target_ref": "knowledge:.magic_novel/terms/foo.json",
                    "kind": "add",
                    "fields": ["not", "an", "object"]
                }
            ]
        }))
        .expect_err("shape validation should reject array fields");

        assert_eq!(err.code, "E_TOOL_SCHEMA_INVALID");
        assert_eq!(err.message, "changes[0].fields must be an object, got array");
    }
}
