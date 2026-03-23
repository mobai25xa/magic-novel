use std::time::Instant;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};
use crate::models::{jvm, AppError, ErrorCode};
use crate::services::jvm::{
    build_doc_from_markdown_blocks, commit_full_document, generate_patch_ops,
    parse_markdown_to_blocks,
};
use crate::services::read_json;
use crate::services::versioning_port::VcCommitPort;

use super::helpers::emit_from_result;
use super::input::{classify_serde_error, take_project_path};
use super::refs::{chapter_path_from_ref, parse_ref};

const TOOL_NAME: &str = "draft_write";
const MAX_MARKDOWN_CHARS: usize = 120_000;
const MAX_DIFF_LINES: usize = 50;
const SNIPPET_CHARS: usize = 800;
const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DraftWriteMode {
    Draft,
    Continue,
    Rewrite,
    Polish,
    Retone,
    Compress,
    Expand,
}

impl DraftWriteMode {
    fn as_str(&self) -> &'static str {
        match self {
            DraftWriteMode::Draft => "draft",
            DraftWriteMode::Continue => "continue",
            DraftWriteMode::Rewrite => "rewrite",
            DraftWriteMode::Polish => "polish",
            DraftWriteMode::Retone => "retone",
            DraftWriteMode::Compress => "compress",
            DraftWriteMode::Expand => "expand",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ContentKind {
    Markdown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftContent {
    kind: ContentKind,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DraftWriteArgs {
    target_ref: String,
    write_mode: DraftWriteMode,
    instruction: String,
    content: DraftContent,
    #[serde(default)]
    constraints: Option<Vec<String>>,
    #[serde(default)]
    context_refs: Option<Vec<String>>,
    #[serde(default)]
    length_target: Option<i64>,
    #[serde(default)]
    dry_run: Option<bool>,
    #[serde(default)]
    idempotency_key: Option<String>,
}

pub(super) fn execute(project_path: &str, mut input: Value, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();

    let project_path = match take_project_path(project_path, &mut input) {
        Ok(p) => p,
        Err(e) => {
            let result = tool_err(
                &call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                &e,
                false,
                FaultDomain::Validation,
                None,
                None,
                None,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let args: DraftWriteArgs = match serde_json::from_value(input) {
        Ok(v) => v,
        Err(err) => {
            let (code, msg) = classify_serde_error(&err);
            let result = tool_err(
                &call_id,
                started,
                code,
                &msg,
                false,
                FaultDomain::Validation,
                None,
                None,
                None,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    if args.instruction.trim().is_empty() {
        let result = tool_err(
            &call_id,
            started,
            "E_TOOL_SCHEMA_INVALID",
            "instruction must be a non-empty string",
            false,
            FaultDomain::Validation,
            None,
            None,
            None,
            None,
            None,
        );
        emit_from_result(&result, "execute");
        return result;
    }

    if let Some(key) = &args.idempotency_key {
        if key.trim().is_empty() || key.len() > MAX_IDEMPOTENCY_KEY_LEN {
            let result = tool_err(
                &call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                "idempotency_key must be 1..=128 characters",
                false,
                FaultDomain::Validation,
                None,
                None,
                None,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    }

    match &args.content.kind {
        ContentKind::Markdown => {}
    }

    let chapter_path = match chapter_path_from_ref(&args.target_ref) {
        Ok(p) => p,
        Err(e) => {
            let result = tool_err(
                &call_id,
                started,
                "E_REF_INVALID",
                &e,
                false,
                FaultDomain::Validation,
                Some(vec![args.target_ref.clone()]),
                None,
                None,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    if let Some(context_refs) = &args.context_refs {
        for r in context_refs {
            if let Err(e) = parse_ref(r) {
                let result = tool_err(
                    &call_id,
                    started,
                    "E_REF_INVALID",
                    &format!("invalid context_ref: {e}"),
                    false,
                    FaultDomain::Validation,
                    Some(vec![args.target_ref.clone()]),
                    None,
                    None,
                    None,
                    None,
                );
                emit_from_result(&result, "execute");
                return result;
            }
        }
    }

    let markdown_chars = args.content.value.chars().count();
    if markdown_chars > MAX_MARKDOWN_CHARS {
        let result = tool_err(
            &call_id,
            started,
            "E_PAYLOAD_TOO_LARGE",
            &format!(
                "content.value too large: {markdown_chars} chars (max {MAX_MARKDOWN_CHARS}); split into smaller writes"
            ),
            false,
            FaultDomain::Validation,
            Some(vec![args.target_ref.clone()]),
            None,
            None,
            None,
            None,
        );
        emit_from_result(&result, "execute");
        return result;
    }

    let chapter_full_path = std::path::PathBuf::from(&project_path)
        .join("manuscripts")
        .join(&chapter_path);
    if !chapter_full_path.exists() {
        let result = tool_err(
            &call_id,
            started,
            "E_REF_NOT_FOUND",
            "target chapter not found",
            false,
            FaultDomain::Io,
            Some(vec![args.target_ref.clone()]),
            None,
            None,
            None,
            None,
        );
        emit_from_result(&result, "execute");
        return result;
    }

    let chapter: crate::models::Chapter = match read_json(&chapter_full_path) {
        Ok(c) => c,
        Err(err) => {
            let result = tool_err(
                &call_id,
                started,
                "E_IO",
                &format!("failed to read chapter: {}", err.message),
                true,
                FaultDomain::Io,
                Some(vec![args.target_ref.clone()]),
                None,
                None,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let vc = crate::services::VersioningService::new();
    let entity_id = format!("chapter:{chapter_path}");
    let head = match vc.get_current_head(&project_path, &entity_id) {
        Ok(h) => h,
        Err(err) => {
            let result = tool_err(
                &call_id,
                started,
                "E_IO",
                &format!("failed to load version head: {}", err.message),
                true,
                FaultDomain::Vc,
                Some(vec![args.target_ref.clone()]),
                None,
                None,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let (md_blocks, _md_diags) = match parse_markdown_to_blocks(&args.content.value) {
        Ok(x) => x,
        Err(err) => {
            let result = tool_err(
                &call_id,
                started,
                "E_TOOL_SCHEMA_INVALID",
                &err.message,
                false,
                FaultDomain::Validation,
                Some(vec![args.target_ref.clone()]),
                Some(head.revision.max(0) as u64),
                None,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let new_doc = build_doc_from_markdown_blocks(&md_blocks);
    let (patch_ops, patch_diags, patch_summary) = generate_patch_ops(&chapter.content, &md_blocks);

    let mut diff_summary = vec![format!("write_mode: {}", args.write_mode.as_str())];
    if let Some(constraints) = args.constraints.as_ref().filter(|c| !c.is_empty()) {
        diff_summary.push(format!("constraints: {}", constraints.len()));
    }
    if let Some(length_target) = args.length_target {
        diff_summary.push(format!("length_target: {length_target}"));
    }
    if patch_diags
        .iter()
        .any(|d| d.level == jvm::DiagnosticLevel::Error)
    {
        diff_summary.push("diff_summary degraded: ambiguous alignment (full rewrite)".to_string());
    }
    diff_summary.extend(patch_summary);
    diff_summary.truncate(MAX_DIFF_LINES);

    let snippet_after = build_snippet_after(&args.content.value);

    let dry_run = args.dry_run.unwrap_or(false);
    if dry_run {
        let mut read_set = vec![args.target_ref.clone()];
        if let Some(context_refs) = &args.context_refs {
            read_set.extend(context_refs.iter().cloned());
        }

        let result = ToolResult {
            ok: true,
            data: Some(json!({
                "accepted": true,
                "mode": "preview",
                "diff_summary": diff_summary,
                "snippet_after": snippet_after,
            })),
            error: None,
            meta: ToolMeta {
                tool: TOOL_NAME.to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: Some(head.revision.max(0) as u64),
                revision_after: None,
                tx_id: None,
                read_set: Some(read_set),
                write_set: Some(vec![]),
            },
        };
        emit_from_result(&result, "execute");
        return result;
    }

    let vc_call_id = build_vc_call_id(&call_id, &chapter_path, args.idempotency_key.as_deref());
    let commit_out = commit_full_document(
        &project_path,
        &chapter_path,
        head.revision,
        &vc_call_id,
        jvm::Actor::Agent,
        new_doc,
        patch_ops,
    );

    let result = match commit_out {
        Ok(out) => {
            let mut read_set = vec![args.target_ref.clone()];
            if let Some(context_refs) = &args.context_refs {
                read_set.extend(context_refs.iter().cloned());
            }

            ToolResult {
                ok: true,
                data: Some(json!({
                    "accepted": true,
                    "mode": "commit",
                    "diff_summary": diff_summary,
                    "tx_id": out.tx_id,
                    "snippet_after": snippet_after,
                })),
                error: None,
                meta: ToolMeta {
                    tool: TOOL_NAME.to_string(),
                    call_id,
                    duration_ms: started.elapsed().as_millis() as u64,
                    revision_before: Some(out.revision_before.max(0) as u64),
                    revision_after: Some(out.revision_after.max(0) as u64),
                    tx_id: Some(out.tx_id),
                    read_set: Some(read_set),
                    write_set: Some(vec![args.target_ref.clone()]),
                },
            }
        }
        Err(err) => {
            let mapped = map_app_error(err);
            ToolResult {
                ok: false,
                data: None,
                error: Some(mapped),
                meta: ToolMeta {
                    tool: TOOL_NAME.to_string(),
                    call_id,
                    duration_ms: started.elapsed().as_millis() as u64,
                    revision_before: Some(head.revision.max(0) as u64),
                    revision_after: None,
                    tx_id: None,
                    read_set: Some(vec![args.target_ref.clone()]),
                    write_set: None,
                },
            }
        }
    };

    emit_from_result(&result, "execute");
    result
}

fn build_vc_call_id(call_id: &str, chapter_path: &str, idempotency_key: Option<&str>) -> String {
    match idempotency_key {
        Some(key) => format!("tool:{TOOL_NAME}:{chapter_path}:{key}"),
        None => call_id.to_string(),
    }
}

fn build_snippet_after(markdown: &str) -> Option<String> {
    let total = markdown.chars().count();
    if total <= SNIPPET_CHARS {
        return None;
    }

    let mut out = String::new();
    for (i, ch) in markdown.chars().enumerate() {
        if i >= SNIPPET_CHARS {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    Some(out)
}

fn map_app_error(err: AppError) -> ToolError {
    let underlying = err
        .details
        .as_ref()
        .and_then(|d| d.get("code"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if matches!(
        underlying,
        "E_VC_CONFLICT_REVISION" | "E_JVM_CONFLICT_REVISION" | "E_VC_LOCK_TIMEOUT"
    ) || matches!(err.code, ErrorCode::Conflict)
    {
        return ToolError {
            code: "E_CONFLICT".to_string(),
            message: "write conflict: target changed; run context_read and retry with the same idempotency_key".to_string(),
            retryable: true,
            fault_domain: FaultDomain::Vc,
            details: err.details,
        };
    }

    if underlying.starts_with("E_JVM_") || matches!(err.code, ErrorCode::SchemaValidationError) {
        return ToolError {
            code: "E_TOOL_SCHEMA_INVALID".to_string(),
            message: err.message,
            retryable: false,
            fault_domain: FaultDomain::Validation,
            details: err.details,
        };
    }

    let (code, fault_domain) = match err.code {
        ErrorCode::NotFound => ("E_REF_NOT_FOUND", FaultDomain::Io),
        ErrorCode::InvalidArgument => ("E_TOOL_SCHEMA_INVALID", FaultDomain::Validation),
        ErrorCode::IoError => ("E_IO", FaultDomain::Io),
        _ => ("E_INTERNAL", FaultDomain::Tool),
    };

    ToolError {
        code: code.to_string(),
        message: err.message,
        retryable: err.recoverable.unwrap_or(false),
        fault_domain,
        details: err.details,
    }
}

fn tool_err(
    call_id: &str,
    started: Instant,
    code: &str,
    message: &str,
    retryable: bool,
    fault_domain: FaultDomain,
    read_set: Option<Vec<String>>,
    revision_before: Option<u64>,
    revision_after: Option<u64>,
    tx_id: Option<String>,
    write_set: Option<Vec<String>>,
) -> ToolResult<Value> {
    ToolResult {
        ok: false,
        data: None,
        error: Some(ToolError {
            code: code.to_string(),
            message: message.to_string(),
            retryable,
            fault_domain,
            details: None,
        }),
        meta: ToolMeta {
            tool: TOOL_NAME.to_string(),
            call_id: call_id.to_string(),
            duration_ms: started.elapsed().as_millis() as u64,
            revision_before,
            revision_after,
            tx_id,
            read_set,
            write_set,
        },
    }
}
