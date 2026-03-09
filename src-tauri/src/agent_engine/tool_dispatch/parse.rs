use crate::agent_tools::contracts::{
    Actor, ContentFormat, CreateInput, CreateKind, DeleteInput, DeleteKind, EditInput, EditOp,
    EditTarget, GrepInput, LsInput, MoveInput, NodeKind, ReadInput, ReadKind, SnapshotBlockInput,
    ViewFormat,
};
use crate::services::load_openai_search_settings;

const READ_FIELDS: &[&str] = &["path", "kind", "view"];
const EDIT_FIELDS: &[&str] = &[
    "path",
    "target",
    "title",
    "summary",
    "status",
    "target_words",
    "tags",
    "pinned_assets",
    "base_revision",
    "snapshot_id",
    "ops",
    "dry_run",
];
const CREATE_FIELDS: &[&str] = &["kind", "title", "volume_path"];
const DELETE_FIELDS: &[&str] = &["kind", "path", "dry_run"];
const MOVE_FIELDS: &[&str] = &[
    "chapter_path",
    "target_volume_path",
    "target_index",
    "dry_run",
];
const LS_FIELDS: &[&str] = &["path", "offset", "limit"];
const GREP_FIELDS: &[&str] = &["query", "mode", "top_k", "scope"];

const E_EDIT_VOLUME_META_FORBIDDEN_FIELDS: &str = "E_EDIT_VOLUME_META_FORBIDDEN_FIELDS";
const E_EDIT_VOLUME_META_FIELDS_REQUIRED: &str = "E_EDIT_VOLUME_META_FIELDS_REQUIRED";
const E_EDIT_CHAPTER_META_FORBIDDEN_FIELDS: &str = "E_EDIT_CHAPTER_META_FORBIDDEN_FIELDS";
const E_EDIT_CHAPTER_META_FIELDS_REQUIRED: &str = "E_EDIT_CHAPTER_META_FIELDS_REQUIRED";
const E_EDIT_CHAPTER_CONTENT_FORBIDDEN_FIELDS: &str = "E_EDIT_CHAPTER_CONTENT_FORBIDDEN_FIELDS";
const E_EDIT_CHAPTER_CONTENT_BASE_REVISION_REQUIRED: &str =
    "E_EDIT_CHAPTER_CONTENT_BASE_REVISION_REQUIRED";
const E_EDIT_CHAPTER_CONTENT_SNAPSHOT_REQUIRED: &str = "E_EDIT_CHAPTER_CONTENT_SNAPSHOT_REQUIRED";
const E_EDIT_CHAPTER_CONTENT_OPS_REQUIRED: &str = "E_EDIT_CHAPTER_CONTENT_OPS_REQUIRED";

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn parser_contract_fields(tool_name: &str) -> Option<&'static [&'static str]> {
    match tool_name {
        "read" => Some(READ_FIELDS),
        "edit" => Some(EDIT_FIELDS),
        "create" => Some(CREATE_FIELDS),
        "delete" => Some(DELETE_FIELDS),
        "move" => Some(MOVE_FIELDS),
        "ls" => Some(LS_FIELDS),
        "grep" => Some(GREP_FIELDS),
        _ => None,
    }
}

pub(super) fn parse_read_input(
    args: &serde_json::Value,
    project_path: &str,
) -> Result<ReadInput, String> {
    reject_unknown_fields(args, READ_FIELDS, "read")?;

    let path = first_non_empty_str(args, &["path"])
        .ok_or_else(|| "read args: missing path".to_string())?
        .to_string();

    if path.is_empty() {
        return Err("read args: path must not be empty".to_string());
    }

    let kind = match first_non_empty_str(args, &["kind"]).unwrap_or("chapter") {
        "volume" => ReadKind::Volume,
        "chapter" => ReadKind::Chapter,
        other => return Err(format!("read args: unsupported kind '{other}'")),
    };

    let default_view = if matches!(kind, ReadKind::Volume) {
        "meta"
    } else {
        "snapshot"
    };
    let view = match first_non_empty_str(args, &["view"]).unwrap_or(default_view) {
        "meta" => ViewFormat::Meta,
        "json" => ViewFormat::Json,
        "snapshot" => ViewFormat::Snapshot,
        other => return Err(format!("read args: unsupported view '{other}'")),
    };

    if matches!(kind, ReadKind::Volume) && !matches!(view, ViewFormat::Meta) {
        return Err("read args: kind=volume only supports view=meta".to_string());
    }

    Ok(ReadInput {
        project_path: project_path.to_string(),
        path,
        kind: Some(kind),
        view,
    })
}

pub(super) fn parse_edit_input(
    args: &serde_json::Value,
    project_path: &str,
    active_chapter_path: Option<&str>,
) -> Result<EditInput, String> {
    reject_unknown_fields(args, EDIT_FIELDS, "edit")?;

    let target = parse_edit_target(args)?;
    let shared = parse_edit_shared_fields(args, active_chapter_path)?;

    match target {
        EditTarget::VolumeMeta => parse_volume_meta_edit(args, project_path, shared),
        EditTarget::ChapterMeta => parse_chapter_meta_edit(args, project_path, shared),
        EditTarget::ChapterContent => parse_chapter_content_edit(args, project_path, shared),
    }
}

struct EditSharedFields {
    path: String,
    dry_run: bool,
    title: Option<String>,
    summary: Option<String>,
    status: Option<String>,
    target_words: Option<i32>,
    tags: Option<Vec<String>>,
    pinned_assets: Option<Vec<crate::models::ChapterAssetRef>>,
}

fn parse_edit_target(args: &serde_json::Value) -> Result<EditTarget, String> {
    match first_non_empty_str(args, &["target"]) {
        Some("volume_meta") => Ok(EditTarget::VolumeMeta),
        Some("chapter_meta") => Ok(EditTarget::ChapterMeta),
        Some("chapter_content") | None => Ok(EditTarget::ChapterContent),
        Some(other) => Err(format!("edit args: unsupported target '{other}'")),
    }
}

fn parse_edit_shared_fields(
    args: &serde_json::Value,
    active_chapter_path: Option<&str>,
) -> Result<EditSharedFields, String> {
    let path = first_non_empty_str(args, &["path"])
        .map(|s| s.to_string())
        .or_else(|| {
            active_chapter_path
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .ok_or_else(|| "edit args: missing path".to_string())?;

    Ok(EditSharedFields {
        path,
        dry_run: first_bool(args, &["dry_run"]).unwrap_or(false),
        title: first_non_empty_str(args, &["title"]).map(ToString::to_string),
        summary: first_non_empty_str(args, &["summary"]).map(ToString::to_string),
        status: first_non_empty_str(args, &["status"]).map(ToString::to_string),
        target_words: first_i64(args, &["target_words"]).map(|v| v as i32),
        tags: parse_tags(args),
        pinned_assets: parse_pinned_assets(args)?,
    })
}

fn parse_volume_meta_edit(
    args: &serde_json::Value,
    project_path: &str,
    shared: EditSharedFields,
) -> Result<EditInput, String> {
    let EditSharedFields {
        path,
        dry_run,
        title,
        summary,
        status,
        target_words,
        tags,
        pinned_assets,
    } = shared;

    if args.get("snapshot_id").is_some() || args.get("ops").is_some() {
        return Err(format!(
            "{E_EDIT_VOLUME_META_FORBIDDEN_FIELDS}: target=volume_meta does not allow snapshot_id/ops"
        ));
    }
    if status.is_some() || target_words.is_some() || tags.is_some() || pinned_assets.is_some() {
        return Err(format!(
            "{E_EDIT_VOLUME_META_FORBIDDEN_FIELDS}: target=volume_meta only allows title/summary"
        ));
    }
    if title.is_none() && summary.is_none() {
        return Err(format!(
            "{E_EDIT_VOLUME_META_FIELDS_REQUIRED}: target=volume_meta requires at least title or summary"
        ));
    }

    Ok(EditInput {
        project_path: project_path.to_string(),
        path,
        target: Some(EditTarget::VolumeMeta),
        title,
        summary,
        status: None,
        target_words: None,
        tags: None,
        pinned_assets: None,
        base_revision: 0,
        snapshot_id: None,
        ops: Vec::new(),
        dry_run,
        actor: Actor::Agent,
    })
}

fn parse_chapter_meta_edit(
    args: &serde_json::Value,
    project_path: &str,
    shared: EditSharedFields,
) -> Result<EditInput, String> {
    let EditSharedFields {
        path,
        dry_run,
        title,
        summary,
        status,
        target_words,
        tags,
        pinned_assets,
    } = shared;

    if args.get("snapshot_id").is_some() || args.get("ops").is_some() {
        return Err(format!(
            "{E_EDIT_CHAPTER_META_FORBIDDEN_FIELDS}: target=chapter_meta does not allow snapshot_id/ops"
        ));
    }
    if title.is_none()
        && summary.is_none()
        && status.is_none()
        && target_words.is_none()
        && tags.is_none()
        && pinned_assets.is_none()
    {
        return Err(format!(
            "{E_EDIT_CHAPTER_META_FIELDS_REQUIRED}: target=chapter_meta requires at least one metadata field"
        ));
    }

    Ok(EditInput {
        project_path: project_path.to_string(),
        path,
        target: Some(EditTarget::ChapterMeta),
        title,
        summary,
        status,
        target_words,
        tags,
        pinned_assets,
        base_revision: 0,
        snapshot_id: None,
        ops: Vec::new(),
        dry_run,
        actor: Actor::Agent,
    })
}

fn parse_chapter_content_edit(
    args: &serde_json::Value,
    project_path: &str,
    shared: EditSharedFields,
) -> Result<EditInput, String> {
    let EditSharedFields {
        path,
        dry_run,
        title,
        summary,
        status,
        target_words,
        tags,
        pinned_assets,
    } = shared;

    if title.is_some()
        || summary.is_some()
        || status.is_some()
        || target_words.is_some()
        || tags.is_some()
        || pinned_assets.is_some()
    {
        return Err(format!(
            "{E_EDIT_CHAPTER_CONTENT_FORBIDDEN_FIELDS}: target=chapter_content only allows base_revision/snapshot_id/ops/dry_run"
        ));
    }

    let base_revision = first_i64(args, &["base_revision"])
        .ok_or_else(|| {
            format!(
                "{E_EDIT_CHAPTER_CONTENT_BASE_REVISION_REQUIRED}: target=chapter_content requires base_revision"
            )
        })?
        .max(0) as u64;

    let snapshot_id = first_non_empty_str(args, &["snapshot_id"])
        .ok_or_else(|| {
            format!(
                "{E_EDIT_CHAPTER_CONTENT_SNAPSHOT_REQUIRED}: target=chapter_content requires snapshot_id"
            )
        })?
        .to_string();

    let ops = parse_edit_ops(args.get("ops").ok_or_else(|| {
        format!("{E_EDIT_CHAPTER_CONTENT_OPS_REQUIRED}: target=chapter_content requires ops")
    })?)?;

    if ops.is_empty() {
        return Err("E_EDIT_OPS_EMPTY: ops must contain at least one operation".to_string());
    }

    Ok(EditInput {
        project_path: project_path.to_string(),
        path,
        target: Some(EditTarget::ChapterContent),
        title: None,
        summary: None,
        status: None,
        target_words: None,
        tags: None,
        pinned_assets: None,
        base_revision,
        snapshot_id: Some(snapshot_id),
        ops,
        dry_run,
        actor: Actor::Agent,
    })
}

pub(super) fn parse_create_input(
    args: &serde_json::Value,
    project_path: &str,
) -> Result<CreateInput, String> {
    reject_unknown_fields(args, CREATE_FIELDS, "create")?;

    let kind = match first_non_empty_str(args, &["kind"]) {
        Some("volume") => CreateKind::Volume,
        Some("chapter") | None => CreateKind::Chapter,
        Some(other) => return Err(format!("create args: unsupported kind '{other}'")),
    };

    let title = first_non_empty_str(args, &["title"])
        .ok_or_else(|| "create args: missing title".to_string())?
        .to_string();

    if title.is_empty() {
        return Err("create args: title must not be empty".to_string());
    }

    let volume_path = first_non_empty_str(args, &["volume_path"]).map(ToString::to_string);

    if matches!(kind, CreateKind::Chapter) && volume_path.is_none() {
        return Err("create args: kind=chapter requires volume_path".to_string());
    }

    let node_kind = match kind {
        CreateKind::Volume => NodeKind::Folder,
        CreateKind::Chapter => NodeKind::File,
    };

    let cwd = volume_path.clone().unwrap_or_else(|| ".".to_string());

    Ok(CreateInput {
        project_path: project_path.to_string(),
        kind: Some(kind),
        volume_path,
        title: Some(title.clone()),
        cwd,
        node_kind,
        name: title,
        content: String::new(),
        content_format: ContentFormat::Text,
        metadata: Default::default(),
    })
}

pub(super) fn parse_delete_input(
    args: &serde_json::Value,
    project_path: &str,
) -> Result<DeleteInput, String> {
    reject_unknown_fields(args, DELETE_FIELDS, "delete")?;

    let kind = match first_non_empty_str(args, &["kind"]) {
        Some("volume") => DeleteKind::Volume,
        Some("chapter") => DeleteKind::Chapter,
        Some(other) => return Err(format!("delete args: unsupported kind '{other}'")),
        None => return Err("delete args: missing kind".to_string()),
    };

    let path = first_non_empty_str(args, &["path"])
        .ok_or_else(|| "delete args: missing path".to_string())?
        .to_string();

    let dry_run = first_bool(args, &["dry_run"]).unwrap_or(false);

    Ok(DeleteInput {
        project_path: project_path.to_string(),
        kind,
        path,
        dry_run,
    })
}

pub(super) fn parse_move_input(
    args: &serde_json::Value,
    project_path: &str,
) -> Result<MoveInput, String> {
    reject_unknown_fields(args, MOVE_FIELDS, "move")?;

    let chapter_path = first_non_empty_str(args, &["chapter_path"])
        .ok_or_else(|| "move args: missing chapter_path".to_string())?
        .to_string();
    let target_volume_path = first_non_empty_str(args, &["target_volume_path"])
        .ok_or_else(|| "move args: missing target_volume_path".to_string())?
        .to_string();
    let target_index = first_i64(args, &["target_index"])
        .ok_or_else(|| "move args: missing target_index".to_string())?;
    if target_index < 0 {
        return Err("move args: target_index must be >= 0".to_string());
    }
    let dry_run = first_bool(args, &["dry_run"]).unwrap_or(false);

    Ok(MoveInput {
        project_path: project_path.to_string(),
        chapter_path,
        target_volume_path,
        target_index: target_index as i32,
        dry_run,
    })
}

pub(super) fn parse_ls_input(
    args: &serde_json::Value,
    project_path: &str,
) -> Result<LsInput, String> {
    reject_unknown_fields(args, LS_FIELDS, "ls")?;

    let cwd = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".")
        .trim()
        .to_string();

    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(30)
        .clamp(1, 200);

    let depth = offset.saturating_add(limit).min(u32::MAX as u64) as u32;

    Ok(LsInput {
        project_path: project_path.to_string(),
        cwd: if cwd.is_empty() { ".".to_string() } else { cwd },
        depth,
        include_hidden: false,
    })
}

pub(super) fn parse_grep_input(
    args: &serde_json::Value,
    project_path: &str,
) -> Result<GrepInput, String> {
    reject_unknown_fields(args, GREP_FIELDS, "grep")?;

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "grep args: missing query".to_string())?
        .trim()
        .to_string();

    if query.is_empty() {
        return Err("grep args: query must not be empty".to_string());
    }

    let requested_mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("keyword");

    let mode = match requested_mode {
        "semantic" => {
            let semantic_enabled = load_openai_search_settings()
                .map(|settings| settings.openai_embedding_enabled)
                .unwrap_or(false);
            if !semantic_enabled {
                return Err(
                    "grep args: mode 'semantic' requires embedding support; use mode='keyword'"
                        .to_string(),
                );
            }
            crate::agent_tools::contracts::GrepMode::Semantic
        }
        "hybrid" => {
            let semantic_enabled = load_openai_search_settings()
                .map(|settings| settings.openai_embedding_enabled)
                .unwrap_or(false);
            if !semantic_enabled {
                return Err(
                    "grep args: mode 'hybrid' requires embedding support; use mode='keyword'"
                        .to_string(),
                );
            }
            crate::agent_tools::contracts::GrepMode::Hybrid
        }
        _ => crate::agent_tools::contracts::GrepMode::Keyword,
    };

    let top_k = args
        .get("top_k")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(10);

    let scope = args.get("scope").and_then(|v| {
        let paths = v
            .get("paths")
            .and_then(|vv| vv.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|it| it.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if paths.is_empty() {
            None
        } else {
            Some(crate::agent_tools::contracts::GrepScope { paths })
        }
    });

    Ok(GrepInput {
        project_path: project_path.to_string(),
        query,
        mode,
        scope,
        top_k,
    })
}

fn first_non_empty_str<'a>(args: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| {
        args.get(*k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    })
}

fn first_bool(args: &serde_json::Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|k| args.get(*k).and_then(|v| v.as_bool()))
}

fn first_i64(args: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|k| args.get(*k).and_then(|v| v.as_i64()))
}

fn parse_tags(args: &serde_json::Value) -> Option<Vec<String>> {
    args.get("tags").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
    })
}

fn parse_pinned_assets(
    args: &serde_json::Value,
) -> Result<Option<Vec<crate::models::ChapterAssetRef>>, String> {
    let Some(value) = args.get("pinned_assets") else {
        return Ok(None);
    };

    if value.is_null() {
        return Ok(Some(Vec::new()));
    }

    let assets: Vec<crate::models::ChapterAssetRef> = serde_json::from_value(value.clone())
        .map_err(|_| "edit args: pinned_assets must be a valid asset ref array".to_string())?;
    Ok(Some(assets))
}

pub(super) fn reject_unknown_fields(
    args: &serde_json::Value,
    fields: &[&str],
    tool: &str,
) -> Result<(), String> {
    let Some(map) = args.as_object() else {
        return Ok(());
    };

    for key in map.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(format!("{tool} args: unknown field '{key}'"));
        }
    }

    Ok(())
}

fn parse_edit_ops(value: &serde_json::Value) -> Result<Vec<EditOp>, String> {
    let arr = value
        .as_array()
        .ok_or_else(|| "edit args: ops must be an array".to_string())?;

    arr.iter()
        .enumerate()
        .map(|(idx, op)| parse_single_edit_op(op, idx))
        .collect()
}

fn parse_single_edit_op(op: &serde_json::Value, idx: usize) -> Result<EditOp, String> {
    let op_name = op
        .get("op")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("edit args: ops[{idx}].op is required"))?;

    match op_name {
        "replace_block" => parse_replace_block_op(op, idx),
        "delete_block" => parse_delete_block_op(op, idx),
        "insert_before" => parse_insert_before_op(op, idx),
        "insert_after" => parse_insert_after_op(op, idx),
        "append_blocks" => parse_append_blocks_op(op, idx),
        "replace_range" => parse_replace_range_op(op, idx),
        other => Err(format!(
            "edit args: ops[{idx}].op unsupported value '{other}'"
        )),
    }
}

fn parse_required_id(op: &serde_json::Value, idx: usize, field: &str) -> Result<String, String> {
    op.get(field)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("edit args: ops[{idx}].{field} is required"))
}

fn parse_replace_block_op(op: &serde_json::Value, idx: usize) -> Result<EditOp, String> {
    let block_id = parse_required_id(op, idx, "block_id")?;
    let markdown = op
        .get("markdown")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("edit args: ops[{idx}].markdown is required"))?
        .to_string();
    Ok(EditOp::ReplaceBlock { block_id, markdown })
}

fn parse_delete_block_op(op: &serde_json::Value, idx: usize) -> Result<EditOp, String> {
    let block_id = parse_required_id(op, idx, "block_id")?;
    Ok(EditOp::DeleteBlock { block_id })
}

fn parse_insert_before_op(op: &serde_json::Value, idx: usize) -> Result<EditOp, String> {
    let block_id = parse_required_id(op, idx, "block_id")?;
    let blocks = parse_block_inputs(op, idx)?;
    Ok(EditOp::InsertBefore { block_id, blocks })
}

fn parse_insert_after_op(op: &serde_json::Value, idx: usize) -> Result<EditOp, String> {
    let block_id = parse_required_id(op, idx, "block_id")?;
    let blocks = parse_block_inputs(op, idx)?;
    Ok(EditOp::InsertAfter { block_id, blocks })
}

fn parse_append_blocks_op(op: &serde_json::Value, idx: usize) -> Result<EditOp, String> {
    let blocks = parse_block_inputs(op, idx)?;
    Ok(EditOp::AppendBlocks { blocks })
}

fn parse_replace_range_op(op: &serde_json::Value, idx: usize) -> Result<EditOp, String> {
    let start_block_id = parse_required_id(op, idx, "start_block_id")?;
    let end_block_id = parse_required_id(op, idx, "end_block_id")?;
    let blocks = parse_block_inputs(op, idx)?;
    Ok(EditOp::ReplaceRange {
        start_block_id,
        end_block_id,
        blocks,
    })
}

fn parse_block_inputs(
    op: &serde_json::Value,
    idx: usize,
) -> Result<Vec<SnapshotBlockInput>, String> {
    let blocks = op
        .get("blocks")
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("edit args: ops[{idx}].blocks must be an array"))?;

    if blocks.is_empty() {
        return Err(format!("edit args: ops[{idx}].blocks must not be empty"));
    }

    let mut out = Vec::with_capacity(blocks.len());
    for (block_idx, block) in blocks.iter().enumerate() {
        let markdown = block
            .get("markdown")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                format!("edit args: ops[{idx}].blocks[{block_idx}].markdown is required")
            })?
            .to_string();
        out.push(SnapshotBlockInput { markdown });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use serde_json::json;

    #[test]
    fn parse_create_volume_ok() {
        let args = json!({
            "kind": "volume",
            "title": "卷一"
        });
        let input = parse_create_input(&args, "D:/tmp/project").expect("create volume parsed");
        assert!(matches!(input.kind, Some(CreateKind::Volume)));
        assert_eq!(input.title.as_deref(), Some("卷一"));
        assert!(input.volume_path.is_none());
    }

    #[test]
    fn parse_create_chapter_without_volume_path_fails() {
        let args = json!({
            "kind": "chapter",
            "title": "第一章"
        });
        let err = parse_create_input(&args, "D:/tmp/project").expect_err("should fail");
        assert!(err.contains("volume_path"));
    }

    #[test]
    fn parse_read_volume_meta_ok() {
        let args = json!({
            "kind": "volume",
            "path": "vol_1",
            "view": "meta"
        });
        let input = parse_read_input(&args, "D:/tmp/project").expect("read parsed");
        assert!(matches!(input.kind, Some(ReadKind::Volume)));
        assert!(matches!(input.view, ViewFormat::Meta));
    }

    #[test]
    fn parse_read_volume_snapshot_fails() {
        let args = json!({
            "kind": "volume",
            "path": "vol_1",
            "view": "snapshot"
        });
        let err = parse_read_input(&args, "D:/tmp/project").expect_err("should fail");
        assert!(err.contains("kind=volume"));
    }

    #[test]
    fn parse_edit_chapter_content_requires_base_revision() {
        let args = json!({
            "target": "chapter_content",
            "path": "vol_1/ch_1.json",
            "snapshot_id": "snap_xxx",
            "ops": [
                {
                    "op": "replace_block",
                    "block_id": "p1",
                    "markdown": "# test"
                }
            ]
        });
        let err = parse_edit_input(&args, "D:/tmp/project", None).expect_err("should fail");
        assert!(err.contains(E_EDIT_CHAPTER_CONTENT_BASE_REVISION_REQUIRED));
        assert!(err.contains("base_revision"));
    }

    #[test]
    fn parse_edit_volume_meta_rejects_snapshot_fields() {
        let args = json!({
            "target": "volume_meta",
            "path": "vol_1",
            "title": "新卷名",
            "snapshot_id": "snap_xxx",
            "ops": []
        });
        let err = parse_edit_input(&args, "D:/tmp/project", None).expect_err("should fail");
        assert!(err.contains(E_EDIT_VOLUME_META_FORBIDDEN_FIELDS));
        assert!(err.contains("volume_meta"));
    }

    #[test]
    fn parse_edit_chapter_meta_requires_at_least_one_field_with_code() {
        let args = json!({
            "target": "chapter_meta",
            "path": "vol_1/ch_1.json"
        });

        let err = parse_edit_input(&args, "D:/tmp/project", None).expect_err("should fail");
        assert!(err.contains(E_EDIT_CHAPTER_META_FIELDS_REQUIRED));
        assert!(err.contains("chapter_meta"));
    }

    #[test]
    fn parse_edit_chapter_content_rejects_metadata_fields_with_code() {
        let args = json!({
            "target": "chapter_content",
            "path": "vol_1/ch_1.json",
            "base_revision": 7,
            "snapshot_id": "snap_7",
            "title": "非法字段",
            "ops": [
                {
                    "op": "replace_block",
                    "block_id": "p1",
                    "markdown": "新的段落"
                }
            ]
        });

        let err = parse_edit_input(&args, "D:/tmp/project", None).expect_err("should fail");
        assert!(err.contains(E_EDIT_CHAPTER_CONTENT_FORBIDDEN_FIELDS));
        assert!(err.contains("chapter_content"));
    }

    #[test]
    fn parse_edit_chapter_content_accepts_ops_payload() {
        let args = json!({
            "target": "chapter_content",
            "path": "vol_1/ch_1.json",
            "base_revision": 7,
            "snapshot_id": "snap_7",
            "ops": [
                {
                    "op": "replace_block",
                    "block_id": "p1",
                    "markdown": "新的段落"
                },
                {
                    "op": "insert_after",
                    "block_id": "p1",
                    "blocks": [
                        { "markdown": "新增段落" }
                    ]
                }
            ]
        });

        let input = parse_edit_input(&args, "D:/tmp/project", None).expect("should parse");
        assert!(matches!(input.target, Some(EditTarget::ChapterContent)));
        assert_eq!(input.base_revision, 7);
        assert_eq!(input.snapshot_id.as_deref(), Some("snap_7"));
        assert_eq!(input.ops.len(), 2);
    }

    #[test]
    fn parse_edit_chapter_content_rejects_empty_ops() {
        let args = json!({
            "target": "chapter_content",
            "path": "vol_1/ch_1.json",
            "base_revision": 7,
            "snapshot_id": "snap_7",
            "ops": []
        });

        let err = parse_edit_input(&args, "D:/tmp/project", None).expect_err("should fail");
        assert!(err.contains("E_EDIT_OPS_EMPTY"));
    }

    #[test]
    fn parse_edit_chapter_content_rejects_unknown_fields() {
        let args = json!({
            "target": "chapter_content",
            "path": "vol_1/ch_1.json",
            "base_revision": 7,
            "snapshot_id": "snap_7",
            "ops": [
                {
                    "op": "replace_block",
                    "block_id": "p1",
                    "markdown": "新的段落"
                }
            ],
            "unexpected_field": "旧段落"
        });

        let err = parse_edit_input(&args, "D:/tmp/project", None).expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("unexpected_field"));
    }

    #[test]
    fn parse_delete_requires_kind() {
        let args = json!({ "path": "vol_1/ch_1.json" });
        let err = parse_delete_input(&args, "D:/tmp/project").expect_err("should fail");
        assert!(err.contains("missing kind"));
    }

    #[test]
    fn parse_delete_rejects_unknown_fields() {
        let args = json!({
            "kind": "chapter",
            "path": "vol_1/ch_1.json",
            "unexpected": true
        });
        let err = parse_delete_input(&args, "D:/tmp/project").expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("unexpected"));
    }

    #[test]
    fn parse_move_requires_target_index() {
        let args = json!({
            "chapter_path": "vol_1/ch_1.json",
            "target_volume_path": "vol_2"
        });
        let err = parse_move_input(&args, "D:/tmp/project").expect_err("should fail");
        assert!(err.contains("target_index"));
    }

    #[test]
    fn parse_move_rejects_unknown_fields() {
        let args = json!({
            "chapter_path": "vol_1/ch_1.json",
            "target_volume_path": "vol_2",
            "target_index": 0,
            "unexpected": true
        });
        let err = parse_move_input(&args, "D:/tmp/project").expect_err("should fail");
        assert!(err.contains("unknown field"));
        assert!(err.contains("unexpected"));
    }

    #[test]
    fn parser_allowlists_match_registered_schema_properties() {
        let context = crate::agent_tools::definition::ToolSchemaContext::default();

        for tool_name in ["read", "edit", "create", "delete", "move", "ls", "grep"] {
            let schema = crate::agent_tools::registry::get_schema(tool_name, &context)
                .unwrap_or_else(|| panic!("missing schema for {tool_name}"));
            let schema_fields: BTreeSet<String> = schema
                .get("properties")
                .and_then(|value| value.as_object())
                .expect("schema properties")
                .keys()
                .cloned()
                .collect();
            let parser_fields: BTreeSet<String> = parser_contract_fields(tool_name)
                .expect("parser fields")
                .iter()
                .map(|field| field.to_string())
                .collect();

            assert_eq!(
                schema_fields, parser_fields,
                "schema/parser mismatch for {tool_name}"
            );
        }
    }
}
