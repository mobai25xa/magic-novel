//! Compatibility forwarding layer for old tool gateway paths.
//! Deprecated in Phase 5. Keep for rollback only; do not add new call sites.

use serde_json::Value;

use crate::agent_tools::contracts::{
    Actor, CreateInput, CreateKind, EditInput, EditOp, EditTarget, LsInput, NodeKind, ReadInput,
    ReadKind, SnapshotBlockInput, ViewFormat,
};
use crate::agent_tools::runtime::{execute_create, execute_edit, execute_ls, execute_read};

fn call_id_or_default(call_id: Option<String>) -> String {
    call_id.unwrap_or_else(|| format!("compat_{}", uuid::Uuid::new_v4()))
}

fn resolve_actor(actor: &str) -> Actor {
    match actor {
        "user" => Actor::User,
        "system" => Actor::System,
        _ => Actor::Agent,
    }
}

fn compat_pre_read(
    project_path: &str,
    chapter_path: &str,
) -> crate::agent_tools::contracts::ToolResult<Value> {
    execute_read(
        ReadInput {
            project_path: project_path.to_string(),
            path: chapter_path.to_string(),
            kind: Some(ReadKind::Chapter),
            view: ViewFormat::Snapshot,
        },
        call_id_or_default(None),
    )
}

fn compat_edit_snapshot_error(
    meta: crate::agent_tools::contracts::ToolMeta,
) -> crate::agent_tools::contracts::ToolResult<Value> {
    crate::agent_tools::contracts::ToolResult {
        ok: false,
        data: None,
        error: Some(crate::agent_tools::contracts::ToolError {
            code: "E_TOOL_SCHEMA_INVALID".to_string(),
            message: "compat_edit failed to get snapshot from pre-read".to_string(),
            retryable: true,
            fault_domain: crate::agent_tools::contracts::FaultDomain::Validation,
            details: None,
        }),
        meta,
    }
}

fn parse_snapshot_context(read_data: &Value) -> (String, u64, Vec<Value>) {
    let snapshot = read_data.get("snapshot").cloned().unwrap_or_default();
    let snapshot_id = snapshot
        .get("snapshot_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let current_revision = read_data
        .get("revision")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0) as u64;
    let blocks = snapshot
        .get("blocks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    (snapshot_id, current_revision, blocks)
}

fn resolve_range_ids(blocks: &[Value]) -> Option<(String, String)> {
    let start = blocks
        .first()
        .and_then(|v| v.get("block_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let end = blocks
        .last()
        .and_then(|v| v.get("block_id"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    if start.is_empty() || end.is_empty() {
        None
    } else {
        Some((start, end))
    }
}

fn build_compat_ops(blocks: &[Value], markdown: String) -> Vec<EditOp> {
    let append = || {
        vec![EditOp::AppendBlocks {
            blocks: vec![SnapshotBlockInput {
                markdown: markdown.clone(),
            }],
        }]
    };

    if blocks.is_empty() {
        return append();
    }

    let Some((start_block_id, end_block_id)) = resolve_range_ids(blocks) else {
        return append();
    };

    vec![EditOp::ReplaceRange {
        start_block_id,
        end_block_id,
        blocks: vec![SnapshotBlockInput { markdown }],
    }]
}

pub fn compat_create(
    project_path: String,
    volume_path: String,
    title: String,
    call_id: Option<String>,
) -> crate::agent_tools::contracts::ToolResult<Value> {
    execute_create(
        CreateInput {
            project_path,
            kind: Some(CreateKind::Chapter),
            volume_path: Some(volume_path.clone()),
            title: Some(title.clone()),
            cwd: volume_path,
            node_kind: NodeKind::File,
            name: title,
            content: String::new(),
            content_format: crate::agent_tools::contracts::ContentFormat::Text,
            metadata: Default::default(),
        },
        call_id_or_default(call_id),
    )
}

pub fn compat_read(
    project_path: String,
    chapter_path: String,
    view: String,
    _include_block_hints: Option<bool>,
    call_id: Option<String>,
) -> crate::agent_tools::contracts::ToolResult<Value> {
    let view = if view == "json" {
        ViewFormat::Json
    } else {
        ViewFormat::Snapshot
    };

    execute_read(
        ReadInput {
            project_path,
            path: chapter_path,
            kind: Some(ReadKind::Chapter),
            view,
        },
        call_id_or_default(call_id),
    )
}

pub fn compat_edit(
    project_path: String,
    chapter_path: String,
    base_revision: i64,
    _mode: String,
    dry_run: bool,
    markdown: String,
    actor: String,
    call_id: Option<String>,
) -> crate::agent_tools::contracts::ToolResult<Value> {
    let actor = resolve_actor(actor.as_str());
    let pre_read = compat_pre_read(&project_path, &chapter_path);

    if !pre_read.ok {
        return pre_read;
    }

    let Some(read_data) = pre_read.data else {
        return compat_edit_snapshot_error(pre_read.meta);
    };

    let (snapshot_id, current_revision, blocks) = parse_snapshot_context(&read_data);
    let ops = build_compat_ops(&blocks, markdown);

    let resolved_base_revision = if base_revision >= 0 {
        base_revision as u64
    } else {
        current_revision
    };

    execute_edit(
        EditInput {
            project_path,
            path: chapter_path,
            target: Some(EditTarget::ChapterContent),
            title: None,
            summary: None,
            status: None,
            target_words: None,
            tags: None,
            pinned_assets: None,
            base_revision: resolved_base_revision,
            snapshot_id: Some(snapshot_id),
            ops,
            dry_run,
            actor,
        },
        call_id_or_default(call_id),
    )
}

pub fn compat_ls(
    project_path: String,
    path: Option<String>,
    call_id: Option<String>,
) -> crate::agent_tools::contracts::ToolResult<Value> {
    execute_ls(
        LsInput {
            project_path,
            cwd: path.unwrap_or_else(|| ".".to_string()),
            depth: 1,
            include_hidden: false,
        },
        call_id_or_default(call_id),
    )
}
