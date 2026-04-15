use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent_tools::tools::r#ref::{
    normalize_project_relative_path, parse_tool_ref, RefError, RefKind, ToolRef,
};
use crate::kernel::search::corpus_extract::extract_tiptap_text;
use crate::models::{Chapter, VolumeMetadata};
use crate::services;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextReadViewMode {
    Compact,
    Full,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextReadSpanKind {
    Head,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReadSpanHead {
    pub kind: ContextReadSpanKind,
    pub chars: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReadArgs {
    pub target_ref: String,
    pub view_mode: Option<ContextReadViewMode>,
    pub budget_chars: Option<u32>,
    pub span: Option<ContextReadSpanHead>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextReadOutput {
    #[serde(rename = "ref")]
    pub ref_: String,
    pub kind: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ContextReadRun {
    pub output: ContextReadOutput,
    pub read_set: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ContextReadError {
    pub code: &'static str,
    pub message: String,
}

pub fn run_context_read(
    project_path: &str,
    args: ContextReadArgs,
) -> Result<ContextReadRun, ContextReadError> {
    let project_path = project_path.trim();
    if project_path.is_empty() {
        return Err(ContextReadError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "missing project_path".to_string(),
        });
    }

    let view_mode = args.view_mode.unwrap_or(ContextReadViewMode::Compact);
    let budget_chars = args.budget_chars.unwrap_or(2000).clamp(0, 50_000) as usize;
    let span_chars = args
        .span
        .as_ref()
        .map(|span| span.chars.unwrap_or(1200).clamp(0, 200_000) as usize);

    let target_ref_raw = args.target_ref.trim();
    if target_ref_raw.is_empty() {
        return Err(ContextReadError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "target_ref is required".to_string(),
        });
    }

    let tref = parse_context_target_ref(target_ref_raw)?;

    match tref.kind {
        RefKind::Chapter => read_chapter(
            project_path,
            &tref.path,
            view_mode,
            budget_chars,
            span_chars,
        )
        .map(|(output, read_set)| ContextReadRun { output, read_set }),
        RefKind::Volume => read_volume(
            project_path,
            &tref.path,
            view_mode,
            budget_chars,
            span_chars,
        )
        .map(|(output, read_set)| ContextReadRun { output, read_set }),
        RefKind::Knowledge => read_knowledge(
            project_path,
            &tref.path,
            view_mode,
            budget_chars,
            span_chars,
        )
        .map(|(output, read_set)| ContextReadRun { output, read_set }),
        RefKind::Book | RefKind::Artifact => Err(ContextReadError {
            code: "E_REF_KIND_UNSUPPORTED",
            message: "context_read supports chapter/volume/knowledge refs in v0".to_string(),
        }),
    }
}

fn read_chapter(
    project_path: &str,
    chapter_ref_path: &str,
    view_mode: ContextReadViewMode,
    budget_chars: usize,
    span_chars: Option<usize>,
) -> Result<(ContextReadOutput, Option<Vec<String>>), ContextReadError> {
    let chapter_rel = normalize_chapter_project_path(chapter_ref_path)?;
    let full = PathBuf::from(project_path).join(&chapter_rel);
    if !full.exists() {
        return Err(ContextReadError {
            code: "E_REF_NOT_FOUND",
            message: "chapter not found".to_string(),
        });
    }

    let chapter: Chapter = services::read_json(&full).map_err(|_| ContextReadError {
        code: "E_IO",
        message: "failed to read chapter".to_string(),
    })?;

    let body = extract_tiptap_text(&chapter.content);
    let header = build_chapter_header(&chapter);

    let default_body_limit = match view_mode {
        ContextReadViewMode::Compact => Some(1200),
        ContextReadViewMode::Full => None,
    };

    let (content, truncated) = assemble_text_output(
        header,
        body,
        span_chars.or(default_body_limit),
        budget_chars,
    );

    let mut meta = serde_json::Map::new();
    meta.insert("title".to_string(), json!(chapter.title));
    meta.insert("chapter_id".to_string(), json!(chapter.id));
    meta.insert("updated_at".to_string(), json!(chapter.updated_at));
    if let Some(status) = chapter
        .status
        .as_ref()
        .map(|s| format!("{s:?}").to_lowercase())
    {
        meta.insert("status".to_string(), json!(status));
    }
    if let Some(summary) = chapter
        .summary
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        meta.insert("summary".to_string(), json!(summary));
    }
    meta.insert(
        "word_count".to_string(),
        json!(chapter
            .counts
            .word_count
            .unwrap_or(chapter.counts.text_length_no_whitespace)),
    );

    Ok((
        ContextReadOutput {
            ref_: format!("chapter:{chapter_rel}"),
            kind: "chapter".to_string(),
            content,
            metadata: Some(serde_json::Value::Object(meta)),
            truncated: truncated.then_some(true),
        },
        Some(vec![format!("chapter:{chapter_rel}")]),
    ))
}

fn read_volume(
    project_path: &str,
    volume_ref_path: &str,
    _view_mode: ContextReadViewMode,
    budget_chars: usize,
    span_chars: Option<usize>,
) -> Result<(ContextReadOutput, Option<Vec<String>>), ContextReadError> {
    let volume_id = normalize_volume_id(volume_ref_path)?;
    let volume_rel = format!("manuscripts/{volume_id}/volume.json");
    let full = PathBuf::from(project_path).join(&volume_rel);
    if !full.exists() {
        return Err(ContextReadError {
            code: "E_REF_NOT_FOUND",
            message: "volume not found".to_string(),
        });
    }

    let volume: VolumeMetadata = services::read_json(&full).map_err(|_| ContextReadError {
        code: "E_IO",
        message: "failed to read volume".to_string(),
    })?;

    let mut body = String::new();
    if let Some(summary) = volume
        .summary
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        body.push_str(summary);
        body.push('\n');
    }
    body.push_str(&format!("chapter_count: {}", volume.chapter_order.len()));

    let header = format!("Volume: {}\n\n", volume.title);

    let (content, truncated) = assemble_text_output(header, body, span_chars, budget_chars);

    let meta = json!({
        "title": volume.title,
        "volume_id": volume.volume_id,
        "updated_at": volume.updated_at,
        "chapter_count": volume.chapter_order.len(),
    });

    Ok((
        ContextReadOutput {
            ref_: format!("volume:manuscripts/{volume_id}"),
            kind: "volume".to_string(),
            content,
            metadata: Some(meta),
            truncated: truncated.then_some(true),
        },
        Some(vec![format!("volume:manuscripts/{volume_id}")]),
    ))
}

fn read_knowledge(
    project_path: &str,
    knowledge_ref_path: &str,
    view_mode: ContextReadViewMode,
    budget_chars: usize,
    span_chars: Option<usize>,
) -> Result<(ContextReadOutput, Option<Vec<String>>), ContextReadError> {
    let virtual_path =
        services::knowledge_paths::normalize_knowledge_virtual_path(knowledge_ref_path)
            .map_err(map_ref_error)?;
    let full = services::knowledge_paths::resolve_knowledge_physical_path(
        Path::new(project_path),
        &virtual_path,
    );
    if !full.exists() || full.is_dir() {
        return Err(ContextReadError {
            code: "E_REF_NOT_FOUND",
            message: "knowledge item not found".to_string(),
        });
    }

    let raw = std::fs::read_to_string(&full).map_err(|_| ContextReadError {
        code: "E_IO",
        message: "failed to read knowledge file".to_string(),
    })?;

    let header = format!("Knowledge: {virtual_path}\n\n");
    let default_body_limit = match view_mode {
        ContextReadViewMode::Compact => Some(1200),
        ContextReadViewMode::Full => None,
    };
    let (content, truncated) =
        assemble_text_output(header, raw, span_chars.or(default_body_limit), budget_chars);

    let meta = json!({
        "path": virtual_path,
    });

    Ok((
        ContextReadOutput {
            ref_: format!("knowledge:{virtual_path}"),
            kind: "knowledge".to_string(),
            content,
            metadata: Some(meta),
            truncated: truncated.then_some(true),
        },
        Some(vec![format!("knowledge:{virtual_path}")]),
    ))
}

fn normalize_chapter_project_path(path: &str) -> Result<String, ContextReadError> {
    let normalized =
        normalize_project_relative_path(path, false).map_err(|err| ContextReadError {
            code: err.code,
            message: err.message,
        })?;

    let rel = normalized
        .strip_prefix("manuscripts/")
        .unwrap_or(normalized.as_str());

    if !rel.ends_with(".json") {
        return Err(ContextReadError {
            code: "E_REF_INVALID",
            message: "chapter ref must point to a .json file".to_string(),
        });
    }

    Ok(format!("manuscripts/{rel}"))
}

fn normalize_volume_id(path: &str) -> Result<String, ContextReadError> {
    let normalized =
        normalize_project_relative_path(path, false).map_err(|err| ContextReadError {
            code: err.code,
            message: err.message,
        })?;

    let trimmed = normalized
        .trim_start_matches("manuscripts/")
        .trim_end_matches("/volume.json")
        .trim_end_matches('/');

    let mut parts = trimmed.split('/').filter(|p| !p.trim().is_empty());
    let Some(volume_id) = parts.next() else {
        return Err(ContextReadError {
            code: "E_REF_INVALID",
            message: "invalid volume ref path".to_string(),
        });
    };
    if parts.next().is_some() {
        return Err(ContextReadError {
            code: "E_REF_INVALID",
            message: "volume ref path must point to a volume directory".to_string(),
        });
    }
    Ok(volume_id.to_string())
}

fn parse_context_target_ref(raw: &str) -> Result<ToolRef, ContextReadError> {
    if services::knowledge_paths::looks_like_knowledge_input(raw) {
        let path = services::knowledge_paths::normalize_knowledge_virtual_path(raw)
            .map_err(map_ref_error)?;
        return Ok(ToolRef {
            kind: RefKind::Knowledge,
            path,
        });
    }

    match parse_tool_ref(raw) {
        Ok(tref) => {
            if tref.kind != RefKind::Knowledge {
                return Ok(tref);
            }

            let path = services::knowledge_paths::normalize_knowledge_virtual_path(&tref.path)
                .map_err(map_ref_error)?;
            Ok(ToolRef {
                kind: RefKind::Knowledge,
                path,
            })
        }
        Err(parse_err) => Err(map_ref_error(parse_err)),
    }
}

fn map_ref_error(err: RefError) -> ContextReadError {
    ContextReadError {
        code: err.code,
        message: err.message,
    }
}

fn build_chapter_header(chapter: &Chapter) -> String {
    let mut header = String::new();
    header.push_str(&format!("Chapter: {}\n", chapter.title.trim()));
    if let Some(status) = chapter
        .status
        .as_ref()
        .map(|s| format!("{s:?}").to_lowercase())
    {
        header.push_str(&format!("status: {status}\n"));
    }
    if let Some(summary) = chapter
        .summary
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        header.push_str(&format!("summary: {summary}\n"));
    }
    header.push('\n');
    header
}

fn assemble_text_output(
    header: String,
    body: String,
    body_head_chars: Option<usize>,
    budget_chars: usize,
) -> (String, bool) {
    let full_body_len = body.chars().count();
    let selected_body = match body_head_chars {
        Some(limit) => take_head_chars(&body, limit),
        None => body,
    };
    let selected_body_len = selected_body.chars().count();

    let mut content = header;
    content.push_str(selected_body.trim());

    let (budgeted, budget_truncated) = truncate_to_chars(&content, budget_chars);
    let truncated = budget_truncated || selected_body_len < full_body_len;
    (budgeted, truncated)
}

fn take_head_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    s.chars().take(max_chars).collect()
}

fn truncate_to_chars(s: &str, max_chars: usize) -> (String, bool) {
    if s.chars().count() <= max_chars {
        return (s.to_string(), false);
    }
    let out = s.chars().take(max_chars).collect::<String>();
    (out, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_project_with_chapter_content(content: &str) -> (tempfile::TempDir, String) {
        let dir = tempdir().expect("temp");
        let project = dir.path().to_path_buf();
        let vol_dir = project.join("manuscripts").join("vol_1");
        services::ensure_dir(&vol_dir).expect("dir");

        let mut vol = VolumeMetadata::new("Vol 1".to_string());
        vol.volume_id = "vol_1".to_string();
        services::write_json(&vol_dir.join("volume.json"), &vol).expect("volume");

        let mut ch = Chapter::new("Ch 1".to_string());
        ch.id = "ch_1".to_string();
        ch.content = serde_json::json!(content);
        services::write_json(&vol_dir.join("ch_1.json"), &ch).expect("chapter");

        (dir, project.to_string_lossy().to_string())
    }

    fn setup_project_with_chapter() -> (tempfile::TempDir, String) {
        setup_project_with_chapter_content("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    }

    #[test]
    fn span_head_limits_body_before_budget() {
        let (_dir, project_path) = setup_project_with_chapter();

        let run = run_context_read(
            &project_path,
            ContextReadArgs {
                target_ref: "chapter:manuscripts/vol_1/ch_1.json".to_string(),
                view_mode: Some(ContextReadViewMode::Full),
                budget_chars: Some(5000),
                span: Some(ContextReadSpanHead {
                    kind: ContextReadSpanKind::Head,
                    chars: Some(10),
                }),
                timeout_ms: None,
            },
        )
        .expect("run");

        assert!(run.output.content.contains("ABCDEFGHIJ"));
        assert!(!run.output.content.contains("KLMNOP"));
        assert_eq!(run.output.truncated, Some(true));
    }

    #[test]
    fn budget_truncates_output() {
        let long_text = "A".repeat(1000);
        let (_dir, project_path) = setup_project_with_chapter_content(&long_text);

        let run = run_context_read(
            &project_path,
            ContextReadArgs {
                target_ref: "chapter:manuscripts/vol_1/ch_1.json".to_string(),
                view_mode: Some(ContextReadViewMode::Full),
                budget_chars: Some(200),
                span: None,
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(run.output.truncated, Some(true));
        assert!(run.output.content.chars().count() <= 200);
    }

    #[test]
    fn knowledge_shorthand_ref_reads_canonical_file_with_canonical_output() {
        let dir = tempdir().expect("temp");
        let project = dir.path();
        let knowledge_dir = project.join(".magic_novel").join("characters");
        services::ensure_dir(&knowledge_dir).expect("knowledge dir");
        std::fs::write(knowledge_dir.join("alice.md"), "# Alice\n\nhero").expect("write");

        let run = run_context_read(
            &project.to_string_lossy(),
            ContextReadArgs {
                target_ref: "characters/alice.md".to_string(),
                view_mode: Some(ContextReadViewMode::Compact),
                budget_chars: Some(5000),
                span: None,
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(
            run.output.ref_,
            "knowledge:.magic_novel/characters/alice.md"
        );
        assert_eq!(
            run.read_set.as_ref().unwrap_or(&Vec::new()),
            &vec!["knowledge:.magic_novel/characters/alice.md".to_string()]
        );
        assert!(run.output.content.contains("Alice"));
    }
}
