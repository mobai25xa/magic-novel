use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::agent_tools::contracts::{FaultDomain, ToolError, ToolMeta, ToolResult};

mod idempotency;
mod support;

use super::helpers::emit_from_result;
use super::input::{classify_serde_error, take_project_path};
use super::refs::{chapter_path_from_ref, chapter_ref, volume_path_from_ref, volume_ref};
use idempotency::{
    fingerprint, load_idempotency_record, save_idempotency_record, IdempotencyRecord,
};
use support::{
    chapter_filename, chapter_volume_path, map_app_error, new_tx_id, tool_err, validate_args,
};

const TOOL_NAME: &str = "structure_edit";
const MAX_SUMMARY_LINES: usize = 50;
const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum StructureOp {
    Create,
    Move,
    Rename,
    Archive,
    Restore,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum StructureNodeType {
    Volume,
    Chapter,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StructureEditArgs {
    op: StructureOp,
    node_type: StructureNodeType,
    #[serde(default)]
    target_ref: Option<String>,
    #[serde(default)]
    parent_ref: Option<String>,
    #[serde(default)]
    position: Option<i64>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    dry_run: Option<bool>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default, rename = "timeout_ms")]
    _timeout_ms: Option<u32>,
}

struct OkOutcome {
    data: serde_json::Value,
    read_set: Option<Vec<String>>,
    write_set: Option<Vec<String>>,
    tx_id: Option<String>,
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
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let args: StructureEditArgs = match serde_json::from_value(input) {
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
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

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
            );
            emit_from_result(&result, "execute");
            return result;
        }
    }

    if let Err(e) = validate_args(&args) {
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
        );
        emit_from_result(&result, "execute");
        return result;
    }

    let dry_run = args.dry_run.unwrap_or(false);

    // Idempotency fast-path for commit mode.
    if !dry_run {
        if let Some(key) = args.idempotency_key.as_deref() {
            if let Some(record) = load_idempotency_record(&project_path, key) {
                if record.fingerprint != fingerprint(&args) {
                    let result = tool_err(
                        &call_id,
                        started,
                        "E_TOOL_SCHEMA_INVALID",
                        "idempotency_key was already used for a different request",
                        false,
                        FaultDomain::Validation,
                        None,
                        None,
                        None,
                        None,
                    );
                    emit_from_result(&result, "execute");
                    return result;
                }

                let result = ToolResult {
                    ok: true,
                    data: Some(record.data),
                    error: None,
                    meta: ToolMeta {
                        tool: TOOL_NAME.to_string(),
                        call_id,
                        duration_ms: started.elapsed().as_millis() as u64,
                        revision_before: None,
                        revision_after: None,
                        tx_id: record.tx_id,
                        read_set: None,
                        write_set: None,
                    },
                };
                emit_from_result(&result, "execute");
                return result;
            }
        }
    }

    let outcome = match args.op {
        StructureOp::Create => exec_create(&project_path, &args, dry_run),
        StructureOp::Move => exec_move(&project_path, &args, dry_run),
        StructureOp::Rename => exec_rename(&project_path, &args, dry_run),
        StructureOp::Archive => exec_archive(&project_path, &args, dry_run),
        StructureOp::Restore => exec_restore(&project_path, &args, dry_run),
    };

    let result = match outcome {
        Ok(ok) => {
            if !dry_run {
                if let Some(key) = args.idempotency_key.as_deref() {
                    let _ = save_idempotency_record(
                        &project_path,
                        key,
                        &IdempotencyRecord {
                            key: key.to_string(),
                            fingerprint: fingerprint(&args),
                            data: ok.data.clone(),
                            tx_id: ok.tx_id.clone(),
                            created_at_ms: chrono::Utc::now().timestamp_millis(),
                        },
                    );
                }
            }

            ToolResult {
                ok: true,
                data: Some(ok.data),
                error: None,
                meta: ToolMeta {
                    tool: TOOL_NAME.to_string(),
                    call_id,
                    duration_ms: started.elapsed().as_millis() as u64,
                    revision_before: None,
                    revision_after: None,
                    tx_id: ok.tx_id,
                    read_set: ok.read_set,
                    write_set: ok.write_set,
                },
            }
        }
        Err(err) => ToolResult {
            ok: false,
            data: None,
            error: Some(err),
            meta: ToolMeta {
                tool: TOOL_NAME.to_string(),
                call_id,
                duration_ms: started.elapsed().as_millis() as u64,
                revision_before: None,
                revision_after: None,
                tx_id: None,
                read_set: None,
                write_set: None,
            },
        },
    };

    emit_from_result(&result, "execute");
    result
}

fn exec_create(
    project_path: &str,
    args: &StructureEditArgs,
    dry_run: bool,
) -> Result<OkOutcome, ToolError> {
    let title = args.title.clone().unwrap_or_default();

    match args.node_type {
        StructureNodeType::Volume => {
            if dry_run {
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": "preview",
                        "impact_summary": [format!("preview: create volume '{title}'")],
                    }),
                    read_set: None,
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            let created = crate::application::command_usecases::volume::create_volume_usecase(
                project_path,
                &title,
            )
            .map_err(map_app_error)?;

            let vref = volume_ref(&created.volume_id);
            let tx_id = new_tx_id();

            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": "commit",
                    "impact_summary": [format!("created volume '{title}' -> {vref}")],
                    "refs": { "after": vref },
                    "tx_id": tx_id,
                }),
                read_set: Some(vec!["book:".to_string()]),
                write_set: Some(vec![volume_ref(&created.volume_id)]),
                tx_id: Some(tx_id),
            })
        }
        StructureNodeType::Chapter => {
            let parent_ref = args.parent_ref.clone().unwrap_or_default();
            let volume_path = volume_path_from_ref(&parent_ref).map_err(|e| ToolError {
                code: "E_REF_INVALID".to_string(),
                message: e,
                retryable: false,
                fault_domain: FaultDomain::Validation,
                details: None,
            })?;

            let volume_dir = std::path::PathBuf::from(project_path)
                .join("manuscripts")
                .join(&volume_path);
            if !volume_dir.join("volume.json").exists() {
                return Err(ToolError {
                    code: "E_REF_NOT_FOUND".to_string(),
                    message: "parent volume not found".to_string(),
                    retryable: false,
                    fault_domain: FaultDomain::Io,
                    details: None,
                });
            }

            if dry_run {
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": "preview",
                        "impact_summary": [format!("preview: create chapter '{title}' under {}", volume_ref(&volume_path))],
                    }),
                    read_set: Some(vec![volume_ref(&volume_path)]),
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            let chapter = crate::application::command_usecases::chapter::create_chapter_usecase(
                project_path,
                &volume_path,
                &title,
            )
            .map_err(map_app_error)?;

            let chapter_path = format!("{volume_path}/{}.json", chapter.id);
            if let Some(pos) = args.position {
                let _ = crate::application::command_usecases::chapter::move_chapter_usecase(
                    project_path,
                    &chapter_path,
                    &volume_path,
                    pos.clamp(0, i32::MAX as i64) as i32,
                )
                .map_err(map_app_error)?;
            }

            let cref = chapter_ref(&chapter_path);
            let tx_id = new_tx_id();

            let mut impact = vec![format!("created chapter '{title}' -> {cref}")];
            if let Some(pos) = args.position {
                impact.push(format!("inserted at position {pos} (0-based)"));
            }
            impact.truncate(MAX_SUMMARY_LINES);

            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": "commit",
                    "impact_summary": impact,
                    "refs": { "after": cref },
                    "tx_id": tx_id,
                }),
                read_set: Some(vec![volume_ref(&volume_path)]),
                write_set: Some(vec![chapter_ref(&chapter_path), volume_ref(&volume_path)]),
                tx_id: Some(tx_id),
            })
        }
    }
}

fn exec_move(
    project_path: &str,
    args: &StructureEditArgs,
    dry_run: bool,
) -> Result<OkOutcome, ToolError> {
    let target_ref = args.target_ref.clone().unwrap_or_default();
    let parent_ref = args.parent_ref.clone().unwrap_or_default();
    let position = args.position.unwrap_or(0);

    let chapter_path = chapter_path_from_ref(&target_ref).map_err(|e| ToolError {
        code: "E_REF_INVALID".to_string(),
        message: e,
        retryable: false,
        fault_domain: FaultDomain::Validation,
        details: None,
    })?;
    let target_volume_path = volume_path_from_ref(&parent_ref).map_err(|e| ToolError {
        code: "E_REF_INVALID".to_string(),
        message: e,
        retryable: false,
        fault_domain: FaultDomain::Validation,
        details: None,
    })?;

    let chapter_full_path = std::path::PathBuf::from(project_path)
        .join("manuscripts")
        .join(&chapter_path);
    if !chapter_full_path.exists() {
        return Err(ToolError {
            code: "E_REF_NOT_FOUND".to_string(),
            message: "target chapter not found".to_string(),
            retryable: false,
            fault_domain: FaultDomain::Io,
            details: None,
        });
    }

    let target_volume_dir = std::path::PathBuf::from(project_path)
        .join("manuscripts")
        .join(&target_volume_path);
    if !target_volume_dir.join("volume.json").exists() {
        return Err(ToolError {
            code: "E_REF_NOT_FOUND".to_string(),
            message: "target volume not found".to_string(),
            retryable: false,
            fault_domain: FaultDomain::Io,
            details: None,
        });
    }

    let before_ref = chapter_ref(&chapter_path);
    let preview_after = chapter_ref(&format!(
        "{}/{}",
        target_volume_path,
        chapter_filename(&chapter_path)
    ));

    if dry_run {
        return Ok(OkOutcome {
            data: json!({
                "accepted": true,
                "mode": "preview",
                "impact_summary": [
                    format!("preview: move {before_ref} -> {preview_after} (position {position}, 0-based)")
                ],
                "refs": { "before": before_ref, "after": preview_after },
            }),
            read_set: Some(vec![before_ref, volume_ref(&target_volume_path)]),
            write_set: Some(vec![]),
            tx_id: None,
        });
    }

    let new_chapter_path = crate::application::command_usecases::chapter::move_chapter_usecase(
        project_path,
        &chapter_path,
        &target_volume_path,
        position.clamp(0, i32::MAX as i64) as i32,
    )
    .map_err(map_app_error)?;

    let after_ref = chapter_ref(&new_chapter_path);
    let tx_id = new_tx_id();

    let source_volume_path = chapter_volume_path(&chapter_path).unwrap_or_default();
    let mut write_set = vec![after_ref.clone()];
    if !source_volume_path.is_empty() {
        write_set.push(volume_ref(&source_volume_path));
    }
    write_set.push(volume_ref(&target_volume_path));

    Ok(OkOutcome {
        data: json!({
            "accepted": true,
            "mode": "commit",
            "impact_summary": [
                format!("moved {before_ref} -> {after_ref} (position {position}, 0-based)")
            ],
            "refs": { "before": before_ref, "after": after_ref },
            "tx_id": tx_id,
        }),
        read_set: Some(vec![before_ref.clone(), volume_ref(&target_volume_path)]),
        write_set: Some(write_set),
        tx_id: Some(tx_id),
    })
}

fn exec_rename(
    project_path: &str,
    args: &StructureEditArgs,
    dry_run: bool,
) -> Result<OkOutcome, ToolError> {
    let target_ref = args.target_ref.clone().unwrap_or_default();
    let title = args.title.clone().unwrap_or_default();

    match args.node_type {
        StructureNodeType::Volume => {
            let volume_path = volume_path_from_ref(&target_ref).map_err(|e| ToolError {
                code: "E_REF_INVALID".to_string(),
                message: e,
                retryable: false,
                fault_domain: FaultDomain::Validation,
                details: None,
            })?;

            let canonical = volume_ref(&volume_path);
            let volume_json = std::path::PathBuf::from(project_path)
                .join("manuscripts")
                .join(&volume_path)
                .join("volume.json");
            if !volume_json.exists() {
                return Err(ToolError {
                    code: "E_REF_NOT_FOUND".to_string(),
                    message: "target volume not found".to_string(),
                    retryable: false,
                    fault_domain: FaultDomain::Io,
                    details: None,
                });
            }

            if !dry_run {
                let _ = crate::application::command_usecases::volume::update_volume_usecase(
                    project_path,
                    &volume_path,
                    Some(title.clone()),
                    None,
                )
                .map_err(map_app_error)?;
            }

            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": if dry_run { "preview" } else { "commit" },
                    "impact_summary": [format!("rename volume {canonical} -> '{title}'")],
                    "refs": { "before": canonical, "after": canonical },
                }),
                read_set: Some(vec![canonical.clone()]),
                write_set: Some(if dry_run { vec![] } else { vec![canonical] }),
                tx_id: None,
            })
        }
        StructureNodeType::Chapter => {
            let chapter_path = chapter_path_from_ref(&target_ref).map_err(|e| ToolError {
                code: "E_REF_INVALID".to_string(),
                message: e,
                retryable: false,
                fault_domain: FaultDomain::Validation,
                details: None,
            })?;

            let canonical = chapter_ref(&chapter_path);
            let chapter_json = std::path::PathBuf::from(project_path)
                .join("manuscripts")
                .join(&chapter_path);
            if !chapter_json.exists() {
                return Err(ToolError {
                    code: "E_REF_NOT_FOUND".to_string(),
                    message: "target chapter not found".to_string(),
                    retryable: false,
                    fault_domain: FaultDomain::Io,
                    details: None,
                });
            }

            if !dry_run {
                let _ =
                    crate::application::command_usecases::chapter::update_chapter_metadata_usecase(
                        project_path,
                        &chapter_path,
                        Some(title.clone()),
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .map_err(map_app_error)?;
            }

            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": if dry_run { "preview" } else { "commit" },
                    "impact_summary": [format!("rename chapter {canonical} -> '{title}'")],
                    "refs": { "before": canonical, "after": canonical },
                }),
                read_set: Some(vec![canonical.clone()]),
                write_set: Some(if dry_run { vec![] } else { vec![canonical] }),
                tx_id: None,
            })
        }
    }
}

fn exec_archive(
    project_path: &str,
    args: &StructureEditArgs,
    dry_run: bool,
) -> Result<OkOutcome, ToolError> {
    let target_ref = args.target_ref.clone().unwrap_or_default();

    match args.node_type {
        StructureNodeType::Volume => {
            let volume_path = volume_path_from_ref(&target_ref).map_err(|e| ToolError {
                code: "E_REF_INVALID".to_string(),
                message: e,
                retryable: false,
                fault_domain: FaultDomain::Validation,
                details: None,
            })?;
            let canonical = volume_ref(&volume_path);

            let volume_dir = std::path::PathBuf::from(project_path)
                .join("manuscripts")
                .join(&volume_path);
            let original_rel = format!("manuscripts/{volume_path}");
            let already_archived = crate::application::command_usecases::recycle::find_recycle_item_id_by_original_rel_path_usecase(project_path, &original_rel)
                .map_err(map_app_error)?
                .is_some();

            if !volume_dir.exists() && !already_archived {
                return Err(ToolError {
                    code: "E_REF_NOT_FOUND".to_string(),
                    message: "target volume not found".to_string(),
                    retryable: false,
                    fault_domain: FaultDomain::Io,
                    details: None,
                });
            }

            if dry_run {
                let msg = if already_archived {
                    format!("preview: archive {canonical} (already archived)")
                } else {
                    format!("preview: archive {canonical}")
                };
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": "preview",
                        "impact_summary": [msg],
                        "refs": { "before": canonical },
                    }),
                    read_set: Some(vec![canonical]),
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            if volume_dir.exists() {
                crate::application::command_usecases::recycle::trash_volume_usecase(
                    project_path,
                    &volume_path,
                )
                .map_err(map_app_error)?;
            }

            let tx_id = new_tx_id();
            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": "commit",
                    "impact_summary": [format!("archived {canonical}")],
                    "refs": { "before": canonical },
                    "tx_id": tx_id,
                }),
                read_set: Some(vec![canonical.clone()]),
                write_set: Some(vec![canonical]),
                tx_id: Some(tx_id),
            })
        }
        StructureNodeType::Chapter => {
            let chapter_path = chapter_path_from_ref(&target_ref).map_err(|e| ToolError {
                code: "E_REF_INVALID".to_string(),
                message: e,
                retryable: false,
                fault_domain: FaultDomain::Validation,
                details: None,
            })?;
            let canonical = chapter_ref(&chapter_path);

            let chapter_file = std::path::PathBuf::from(project_path)
                .join("manuscripts")
                .join(&chapter_path);
            let original_rel = format!("manuscripts/{chapter_path}");
            let already_archived = crate::application::command_usecases::recycle::find_recycle_item_id_by_original_rel_path_usecase(project_path, &original_rel)
                .map_err(map_app_error)?
                .is_some();

            if !chapter_file.exists() && !already_archived {
                return Err(ToolError {
                    code: "E_REF_NOT_FOUND".to_string(),
                    message: "target chapter not found".to_string(),
                    retryable: false,
                    fault_domain: FaultDomain::Io,
                    details: None,
                });
            }

            if dry_run {
                let msg = if already_archived {
                    format!("preview: archive {canonical} (already archived)")
                } else {
                    format!("preview: archive {canonical}")
                };
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": "preview",
                        "impact_summary": [msg],
                        "refs": { "before": canonical },
                    }),
                    read_set: Some(vec![canonical]),
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            if chapter_file.exists() {
                crate::application::command_usecases::recycle::trash_chapter_usecase(
                    project_path,
                    &chapter_path,
                )
                .map_err(map_app_error)?;
            }

            let tx_id = new_tx_id();
            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": "commit",
                    "impact_summary": [format!("archived {canonical}")],
                    "refs": { "before": canonical },
                    "tx_id": tx_id,
                }),
                read_set: Some(vec![canonical.clone()]),
                write_set: Some(vec![canonical]),
                tx_id: Some(tx_id),
            })
        }
    }
}

fn exec_restore(
    project_path: &str,
    args: &StructureEditArgs,
    dry_run: bool,
) -> Result<OkOutcome, ToolError> {
    let target_ref = args.target_ref.clone().unwrap_or_default();

    match args.node_type {
        StructureNodeType::Volume => {
            let volume_path = volume_path_from_ref(&target_ref).map_err(|e| ToolError {
                code: "E_REF_INVALID".to_string(),
                message: e,
                retryable: false,
                fault_domain: FaultDomain::Validation,
                details: None,
            })?;
            let canonical = volume_ref(&volume_path);

            let volume_dir = std::path::PathBuf::from(project_path)
                .join("manuscripts")
                .join(&volume_path);
            if volume_dir.exists() {
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": if dry_run { "preview" } else { "commit" },
                        "impact_summary": [format!("restore {canonical} (already present)")],
                        "refs": { "after": canonical },
                    }),
                    read_set: Some(vec![canonical]),
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            let original_rel = format!("manuscripts/{volume_path}");
            let item_id =
                crate::application::command_usecases::recycle::find_recycle_item_id_by_original_rel_path_usecase(
                    project_path,
                    &original_rel,
                )
                .map_err(map_app_error)?
                .ok_or_else(|| ToolError {
                    code: "E_REF_NOT_FOUND".to_string(),
                    message: "archived volume not found in recycle bin".to_string(),
                    retryable: false,
                    fault_domain: FaultDomain::Io,
                    details: None,
                })?;

            if dry_run {
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": "preview",
                        "impact_summary": [format!("preview: restore {canonical}")],
                        "refs": { "after": canonical },
                    }),
                    read_set: Some(vec![canonical]),
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            crate::application::command_usecases::recycle::restore_recycle_item_usecase(
                project_path,
                &item_id,
            )
            .map_err(map_app_error)?;

            let tx_id = new_tx_id();
            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": "commit",
                    "impact_summary": [format!("restored {canonical}")],
                    "refs": { "after": canonical },
                    "tx_id": tx_id,
                }),
                read_set: Some(vec![canonical.clone()]),
                write_set: Some(vec![canonical]),
                tx_id: Some(tx_id),
            })
        }
        StructureNodeType::Chapter => {
            let chapter_path = chapter_path_from_ref(&target_ref).map_err(|e| ToolError {
                code: "E_REF_INVALID".to_string(),
                message: e,
                retryable: false,
                fault_domain: FaultDomain::Validation,
                details: None,
            })?;
            let canonical = chapter_ref(&chapter_path);

            let chapter_file = std::path::PathBuf::from(project_path)
                .join("manuscripts")
                .join(&chapter_path);
            if chapter_file.exists() {
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": if dry_run { "preview" } else { "commit" },
                        "impact_summary": [format!("restore {canonical} (already present)")],
                        "refs": { "after": canonical },
                    }),
                    read_set: Some(vec![canonical]),
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            let original_rel = format!("manuscripts/{chapter_path}");
            let item_id =
                crate::application::command_usecases::recycle::find_recycle_item_id_by_original_rel_path_usecase(
                    project_path,
                    &original_rel,
                )
                .map_err(map_app_error)?
                .ok_or_else(|| ToolError {
                    code: "E_REF_NOT_FOUND".to_string(),
                    message: "archived chapter not found in recycle bin".to_string(),
                    retryable: false,
                    fault_domain: FaultDomain::Io,
                    details: None,
                })?;

            if dry_run {
                return Ok(OkOutcome {
                    data: json!({
                        "accepted": true,
                        "mode": "preview",
                        "impact_summary": [format!("preview: restore {canonical}")],
                        "refs": { "after": canonical },
                    }),
                    read_set: Some(vec![canonical]),
                    write_set: Some(vec![]),
                    tx_id: None,
                });
            }

            crate::application::command_usecases::recycle::restore_recycle_item_usecase(
                project_path,
                &item_id,
            )
            .map_err(map_app_error)?;

            let tx_id = new_tx_id();
            Ok(OkOutcome {
                data: json!({
                    "accepted": true,
                    "mode": "commit",
                    "impact_summary": [format!("restored {canonical}")],
                    "refs": { "after": canonical },
                    "tx_id": tx_id,
                }),
                read_set: Some(vec![canonical.clone()]),
                write_set: Some(vec![canonical]),
                tx_id: Some(tx_id),
            })
        }
    }
}
