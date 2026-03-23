use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent_tools::tools::r#ref::{parse_tool_ref, RefError, RefKind};
use crate::services;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeReadViewMode {
    Compact,
    Full,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeReadArgs {
    pub knowledge_type: Option<String>,
    pub item_ref: Option<String>,
    pub query: Option<String>,
    pub view_mode: Option<KnowledgeReadViewMode>,
    pub top_k: Option<u32>,
    pub budget_chars: Option<u32>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeReadItem {
    #[serde(rename = "ref")]
    pub ref_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeReadOutput {
    pub items: Vec<KnowledgeReadItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeReadRun {
    pub output: KnowledgeReadOutput,
    pub read_set: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeReadError {
    pub code: &'static str,
    pub message: String,
}

const DEFAULT_TOP_K: u32 = 10;
const DEFAULT_BUDGET_CHARS: u32 = 2000;
const MAX_TOP_K: u32 = 50;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

pub fn run_knowledge_read(
    project_path: &str,
    args: KnowledgeReadArgs,
) -> Result<KnowledgeReadRun, KnowledgeReadError> {
    let project_path = project_path.trim();
    if project_path.is_empty() {
        return Err(KnowledgeReadError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "missing project_path".to_string(),
        });
    }

    let item_ref = args
        .item_ref
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let query = args
        .query
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let view_mode = args.view_mode.unwrap_or(KnowledgeReadViewMode::Compact);
    let top_k = args.top_k.unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K) as usize;
    let budget_chars = args
        .budget_chars
        .unwrap_or(DEFAULT_BUDGET_CHARS)
        .clamp(0, 50_000) as usize;

    let type_dir = args
        .knowledge_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|ty| {
            knowledge_type_dir(ty).ok_or_else(|| KnowledgeReadError {
                code: "E_TOOL_SCHEMA_INVALID",
                message: "knowledge_type is invalid".to_string(),
            })
        })
        .transpose()?;

    if let Some(item_ref) = item_ref {
        let (item, read_set, truncated) =
            read_by_ref(project_path, item_ref, type_dir, view_mode, budget_chars)?;
        return Ok(KnowledgeReadRun {
            output: KnowledgeReadOutput {
                items: vec![item],
                truncated: truncated.then_some(true),
            },
            read_set: Some(read_set),
        });
    }

    if let Some(query) = query {
        let (items, read_set, truncated) = search_by_query(
            project_path,
            query,
            type_dir,
            view_mode,
            top_k,
            budget_chars,
        )?;
        return Ok(KnowledgeReadRun {
            output: KnowledgeReadOutput {
                items,
                truncated: truncated.then_some(true),
            },
            read_set: Some(read_set),
        });
    }

    if let Some(dir) = type_dir {
        let (items, read_set, truncated) = list_by_type(project_path, dir, top_k)?;
        return Ok(KnowledgeReadRun {
            output: KnowledgeReadOutput {
                items,
                truncated: truncated.then_some(true),
            },
            read_set: Some(read_set),
        });
    }

    return Err(KnowledgeReadError {
        code: "E_TOOL_SCHEMA_INVALID",
        message: "item_ref or query is required".to_string(),
    });
}

fn list_by_type(
    project_path: &str,
    type_dir: &'static str,
    top_k: usize,
) -> Result<(Vec<KnowledgeReadItem>, Vec<String>, bool), KnowledgeReadError> {
    let roots = services::knowledge_paths::knowledge_read_roots(Path::new(project_path));
    if roots.is_empty() {
        return Ok((
            Vec::new(),
            vec!["knowledge:.magic_novel".to_string()],
            false,
        ));
    }

    let rel_paths = collect_unique_files(&roots, Some(type_dir), top_k.saturating_add(1));
    let truncated = rel_paths.len() > top_k;
    let rel_paths = rel_paths.into_iter().take(top_k).collect::<Vec<_>>();

    let mut items = Vec::new();
    let mut read_set = Vec::new();

    for rel_path in rel_paths {
        let virtual_path = to_virtual_path_from_rel(&rel_path)?;
        let physical = physical_path_for_virtual(Path::new(project_path), &virtual_path);
        let raw = std::fs::read_to_string(&physical).unwrap_or_default();
        let (title, summary) = extract_title_summary(&raw);

        read_set.push(format!("knowledge:{virtual_path}"));
        items.push(KnowledgeReadItem {
            ref_: format!("knowledge:{virtual_path}"),
            title,
            summary,
            snippet: None,
            path: Some(virtual_path),
        });
    }

    if items.is_empty() {
        read_set.push("knowledge:.magic_novel".to_string());
    }

    Ok((items, read_set, truncated))
}

fn collect_files(
    base: &Path,
    dir: &Path,
    max: usize,
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    if out.len() >= max {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for entry in entries {
        if out.len() >= max {
            break;
        }

        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            collect_files(base, &path, max, out, seen);
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if !matches_extension(ext) {
                continue;
            }
        } else {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_FILE_BYTES {
            continue;
        }

        let rel = path
            .strip_prefix(base)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string())
            .replace('\\', "/");
        if seen.insert(rel.clone()) {
            out.push(rel);
        }
    }
}

fn knowledge_type_dir(ty: &str) -> Option<&'static str> {
    match ty.trim() {
        "character" => Some("characters"),
        "location" => Some("locations"),
        "organization" => Some("organizations"),
        "rule" => Some("rules"),
        "term" => Some("terms"),
        "plotline" => Some("plotlines"),
        "style_rule" => Some("style_rules"),
        "source" => Some("sources"),
        "chapter_summary" => Some("chapter_summaries"),
        "recent_fact" => Some("recent_facts"),
        "foreshadow" => Some("foreshadow"),
        _ => None,
    }
}

fn to_virtual_path_from_rel(rel: &str) -> Result<String, KnowledgeReadError> {
    services::knowledge_paths::normalize_knowledge_virtual_path(&format!(
        ".magic_novel/{}",
        rel.trim_start_matches('/')
    ))
    .map_err(|err| KnowledgeReadError {
        code: "E_INTERNAL",
        message: err.message,
    })
}

fn physical_path_for_virtual(project_path: &Path, virtual_path: &str) -> PathBuf {
    services::knowledge_paths::map_virtual_magic_novel_path(project_path, virtual_path)
}

fn read_by_ref(
    project_path: &str,
    item_ref: &str,
    type_dir: Option<&'static str>,
    view_mode: KnowledgeReadViewMode,
    budget_chars: usize,
) -> Result<(KnowledgeReadItem, Vec<String>, bool), KnowledgeReadError> {
    let virtual_path = parse_knowledge_item_ref(item_ref)?;

    if let Some(dir) = type_dir {
        let prefix = format!(".magic_novel/{dir}/");
        if !virtual_path.starts_with(&prefix) {
            return Err(KnowledgeReadError {
                code: "E_TOOL_SCHEMA_INVALID",
                message: "item_ref does not match knowledge_type filter".to_string(),
            });
        }
    }

    let physical = physical_path_for_virtual(Path::new(project_path), &virtual_path);
    if !physical.exists() || physical.is_dir() {
        return Err(KnowledgeReadError {
            code: "E_REF_NOT_FOUND",
            message: "knowledge item not found".to_string(),
        });
    }

    let meta = std::fs::metadata(&physical).map_err(|_| KnowledgeReadError {
        code: "E_IO",
        message: "failed to stat knowledge item".to_string(),
    })?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(KnowledgeReadError {
            code: "E_PAYLOAD_TOO_LARGE",
            message: "knowledge item is too large".to_string(),
        });
    }

    let raw = std::fs::read_to_string(&physical).map_err(|_| KnowledgeReadError {
        code: "E_IO",
        message: "failed to read knowledge item".to_string(),
    })?;

    let (title, summary) = extract_title_summary(&raw);

    let (snippet, truncated) = match view_mode {
        KnowledgeReadViewMode::Compact => (None, false),
        KnowledgeReadViewMode::Full => {
            let (excerpt, truncated) = truncate_to_chars(raw.trim(), budget_chars);
            (Some(excerpt), truncated)
        }
    };

    Ok((
        KnowledgeReadItem {
            ref_: format!("knowledge:{virtual_path}"),
            title,
            summary,
            snippet,
            path: Some(virtual_path.clone()),
        },
        vec![format!("knowledge:{virtual_path}")],
        truncated,
    ))
}

fn search_by_query(
    project_path: &str,
    query: &str,
    type_dir: Option<&'static str>,
    view_mode: KnowledgeReadViewMode,
    top_k: usize,
    budget_chars: usize,
) -> Result<(Vec<KnowledgeReadItem>, Vec<String>, bool), KnowledgeReadError> {
    let roots = services::knowledge_paths::knowledge_read_roots(Path::new(project_path));
    if roots.is_empty() {
        return Ok((
            Vec::new(),
            vec!["knowledge:.magic_novel".to_string()],
            false,
        ));
    }

    let mut seen = HashSet::new();
    let mut matches: Vec<(String, String)> = Vec::new();
    for root in &roots {
        let search_root = if let Some(dir) = type_dir {
            root.join(dir)
        } else {
            root.clone()
        };

        if !search_root.exists() || !search_root.is_dir() {
            continue;
        }

        collect_matches(root, &search_root, query, &mut matches, &mut seen);
        if matches.len() >= top_k {
            break;
        }
    }
    matches.truncate(top_k);

    let mut items = Vec::new();
    let mut read_set = Vec::new();
    let mut any_truncated = false;

    for (rel_path, hit_snippet) in matches {
        let virtual_path = to_virtual_path_from_rel(&rel_path)?;
        let physical = physical_path_for_virtual(Path::new(project_path), &virtual_path);
        let raw = std::fs::read_to_string(&physical).unwrap_or_default();
        let (title, summary) = extract_title_summary(&raw);

        let (snippet, truncated) = match view_mode {
            KnowledgeReadViewMode::Compact => (Some(limit_snippet(&hit_snippet, 400)), false),
            KnowledgeReadViewMode::Full => {
                let (excerpt, truncated) = truncate_to_chars(raw.trim(), budget_chars);
                (Some(excerpt), truncated)
            }
        };

        any_truncated |= truncated;
        read_set.push(format!("knowledge:{virtual_path}"));
        items.push(KnowledgeReadItem {
            ref_: format!("knowledge:{virtual_path}"),
            title,
            summary,
            snippet,
            path: Some(virtual_path),
        });
    }

    if items.is_empty() {
        read_set.push("knowledge:.magic_novel".to_string());
    }

    Ok((items, read_set, any_truncated))
}

fn collect_matches(
    base: &Path,
    dir: &Path,
    query: &str,
    matches: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let query_lower = query.trim().to_lowercase();
    if query_lower.is_empty() {
        return;
    }

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_matches(base, &path, query, matches, seen);
            continue;
        }

        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if !matches_extension(ext) {
                continue;
            }
        } else {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_FILE_BYTES {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };

        let lower = content.to_lowercase();
        if !lower.contains(&query_lower) {
            continue;
        }

        let rel = path
            .strip_prefix(base)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string())
            .replace('\\', "/");
        if !seen.insert(rel.clone()) {
            continue;
        }

        if let Some(snippet) = extract_snippet(&content, &lower, &query_lower) {
            matches.push((rel, snippet));
        } else {
            matches.push((rel, limit_snippet(&content, 200)));
        }
    }
}

fn collect_unique_files(roots: &[PathBuf], type_dir: Option<&str>, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for root in roots {
        let search_root = if let Some(dir) = type_dir {
            root.join(dir)
        } else {
            root.clone()
        };

        if !search_root.exists() || !search_root.is_dir() {
            continue;
        }

        collect_files(root, &search_root, max, &mut out, &mut seen);
        if out.len() >= max {
            break;
        }
    }

    out
}

fn parse_knowledge_item_ref(raw: &str) -> Result<String, KnowledgeReadError> {
    if services::knowledge_paths::looks_like_knowledge_input(raw) {
        return services::knowledge_paths::normalize_knowledge_virtual_path(raw)
            .map_err(map_ref_error);
    }

    match parse_tool_ref(raw) {
        Ok(tref) => {
            if tref.kind != RefKind::Knowledge {
                return Err(KnowledgeReadError {
                    code: "E_REF_KIND_UNSUPPORTED",
                    message: "item_ref must be a knowledge ref".to_string(),
                });
            }

            services::knowledge_paths::normalize_knowledge_virtual_path(&tref.path)
                .map_err(map_ref_error)
        }
        Err(parse_err) => Err(map_ref_error(parse_err)),
    }
}

fn map_ref_error(err: RefError) -> KnowledgeReadError {
    KnowledgeReadError {
        code: err.code,
        message: err.message,
    }
}

fn matches_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "md" | "json" | "txt" | "yaml" | "yml"
    )
}

fn extract_snippet(content: &str, content_lower: &str, query_lower: &str) -> Option<String> {
    let pos = content_lower.find(query_lower)?;
    let start = pos.saturating_sub(100);
    let end = (pos + query_lower.len() + 100).min(content.len());

    let start = content[..start]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let end = content[end..]
        .char_indices()
        .next()
        .map(|(idx, _)| end + idx)
        .unwrap_or(content.len());

    Some(content[start..end].trim().to_string())
}

fn limit_snippet(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let head = trimmed.chars().take(max_chars).collect::<String>();
    format!("{head}…")
}

fn truncate_to_chars(s: &str, max_chars: usize) -> (String, bool) {
    if s.chars().count() <= max_chars {
        return (s.to_string(), false);
    }
    let out = s.chars().take(max_chars).collect::<String>();
    (out, true)
}

fn extract_title_summary(raw: &str) -> (Option<String>, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (None, None);
    }

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some((title, summary)) = extract_json_title_summary(&value) {
                return (title, summary);
            }
        }
    }

    // Markdown: first heading as title.
    if let Some(line) = trimmed.lines().find(|l| !l.trim().is_empty()) {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix('#') {
            let title = rest.trim_start_matches('#').trim();
            if !title.is_empty() {
                let body = trimmed.lines().skip(1).collect::<Vec<_>>().join("\n");
                let summary = body
                    .trim()
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .map(|s| limit_snippet(s, 400));
                return (Some(title.to_string()), summary);
            }
        }
    }

    (None, Some(limit_snippet(trimmed, 400)))
}

fn extract_json_title_summary(
    value: &serde_json::Value,
) -> Option<(Option<String>, Option<String>)> {
    let obj = value.as_object()?;

    let fields = obj.get("fields").and_then(|v| v.as_object()).unwrap_or(obj);
    let title = find_first_string(
        fields,
        &[
            "title",
            "name",
            "chapter_title",
            "seed_ref",
            "chapter_locator",
        ],
    );
    let summary = find_first_string(
        fields,
        &["summary", "description", "current_notes", "notes"],
    )
    .map(|s| limit_snippet(&s, 400));

    if title.is_some() || summary.is_some() {
        return Some((title, summary));
    }

    None
}

fn find_first_string(
    obj: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    for k in keys {
        if let Some(s) = obj.get(*k).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_item_ref_and_query_is_invalid() {
        let dir = tempdir().expect("temp");
        let project_path = dir.path().to_string_lossy().to_string();
        let err = run_knowledge_read(
            &project_path,
            KnowledgeReadArgs {
                knowledge_type: None,
                item_ref: None,
                query: None,
                view_mode: None,
                top_k: None,
                budget_chars: None,
                timeout_ms: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "E_TOOL_SCHEMA_INVALID");
    }

    #[test]
    fn full_view_is_budgeted_and_truncated() {
        let dir = tempdir().expect("temp");
        let project = dir.path();
        std::fs::create_dir_all(project.join(".magic_novel").join("terms")).expect("dir");
        std::fs::write(
            project.join(".magic_novel").join("terms").join("foo.json"),
            r#"{"fields":{"title":"Foo","summary":"abcdefghijklmnopqrstuvwxyz"}}"#,
        )
        .expect("write");

        let run = run_knowledge_read(
            &project.to_string_lossy(),
            KnowledgeReadArgs {
                knowledge_type: Some("term".to_string()),
                item_ref: Some("knowledge:.magic_novel/terms/foo.json".to_string()),
                query: None,
                view_mode: Some(KnowledgeReadViewMode::Full),
                top_k: None,
                budget_chars: Some(10),
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(run.output.items.len(), 1);
        assert_eq!(
            run.output.items[0].ref_,
            "knowledge:.magic_novel/terms/foo.json"
        );
        assert_eq!(run.output.truncated, Some(true));
        assert!(
            run.output.items[0]
                .snippet
                .as_deref()
                .unwrap_or("")
                .chars()
                .count()
                <= 10
        );
    }

    #[test]
    fn knowledge_type_only_lists_items() {
        let dir = tempdir().expect("temp");
        let project = dir.path();
        std::fs::create_dir_all(project.join(".magic_novel").join("terms")).expect("dir");
        std::fs::write(
            project.join(".magic_novel").join("terms").join("foo.json"),
            r#"{"fields":{"title":"Foo","summary":"foo summary"}}"#,
        )
        .expect("write");
        std::fs::write(
            project.join(".magic_novel").join("terms").join("bar.md"),
            "# Bar\n\nbar summary",
        )
        .expect("write");

        let run = run_knowledge_read(
            &project.to_string_lossy(),
            KnowledgeReadArgs {
                knowledge_type: Some("term".to_string()),
                item_ref: None,
                query: None,
                view_mode: None,
                top_k: Some(10),
                budget_chars: None,
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(run.output.items.len(), 2);
        assert!(run
            .output
            .items
            .iter()
            .any(|i| i.title.as_deref() == Some("Foo")));
        assert!(run
            .output
            .items
            .iter()
            .any(|i| i.title.as_deref() == Some("Bar")));
        assert!(run
            .read_set
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .all(|r| r.starts_with("knowledge:")));
    }

    #[test]
    fn shorthand_item_ref_reads_canonical_file() {
        let dir = tempdir().expect("temp");
        let project = dir.path();
        std::fs::create_dir_all(project.join(".magic_novel").join("characters")).expect("dir");
        std::fs::write(
            project
                .join(".magic_novel")
                .join("characters")
                .join("alice.md"),
            "# Alice\n\nhero",
        )
        .expect("write");

        let run = run_knowledge_read(
            &project.to_string_lossy(),
            KnowledgeReadArgs {
                knowledge_type: Some("character".to_string()),
                item_ref: Some("characters/alice.md".to_string()),
                query: None,
                view_mode: Some(KnowledgeReadViewMode::Full),
                top_k: None,
                budget_chars: Some(200),
                timeout_ms: None,
            },
        )
        .expect("run");

        assert_eq!(run.output.items.len(), 1);
        assert_eq!(
            run.output.items[0].ref_,
            "knowledge:.magic_novel/characters/alice.md"
        );
        assert_eq!(
            run.output.items[0].path.as_deref(),
            Some(".magic_novel/characters/alice.md")
        );
    }
}
