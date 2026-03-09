use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use regex::{Regex, RegexBuilder};
use serde_json::json;

use crate::agent_tools::contracts::{GrepHit, GrepMode, GrepOutput, GrepScope};
use crate::kernel::search::corpus_extract::extract_tiptap_text;
use crate::kernel::search::keyword::{build_snippet, find_keyword, score_keyword};
use crate::models::{AppError, Chapter};
use crate::services::{list_dirs, list_files, read_json};

use super::keyword_assets::grep_assets_keyword;
use crate::application::search_usecases::bm25::query::try_grep_bm25;
use crate::application::search_usecases::index::corpus::{fingerprint_corpus, scan_corpus};
use crate::application::search_usecases::index::scope::{in_scope, normalize_scope_prefixes};

const MANUSCRIPTS_DIR: &str = "manuscripts";
const VOLUME_FILE: &str = "volume.json";
const MATCH_COUNT_CAP: u32 = 64;
const SNIPPET_CONTEXT_CHARS: usize = 60;

#[derive(Debug, Clone)]
struct ChapterFileInfo {
    chapter_path: String,
    full_path: PathBuf,
    modified_ms: i64,
}

#[derive(Debug, Clone)]
struct CachedChapter {
    modified_ms: i64,
    chapter_id: String,
    title: String,
    text: Arc<String>,
}

#[derive(Debug, Clone)]
struct ChapterDoc {
    chapter_path: String,
    chapter_id: String,
    title: String,
    text: Arc<String>,
}

#[derive(Debug, Default)]
struct ProjectChapterCache {
    chapters: HashMap<String, CachedChapter>,
}

static PROJECT_CACHE: OnceLock<Mutex<HashMap<String, ProjectChapterCache>>> = OnceLock::new();

pub fn grep_keyword(
    project_path: &str,
    query: &str,
    scope: Option<&GrepScope>,
    top_k: u32,
) -> Result<GrepOutput, AppError> {
    grep_keyword_with_mode(project_path, query, scope, top_k, GrepMode::Keyword)
}

pub(crate) fn grep_keyword_with_mode(
    project_path: &str,
    query: &str,
    scope: Option<&GrepScope>,
    top_k: u32,
    mode: GrepMode,
) -> Result<GrepOutput, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(GrepOutput {
            hits: vec![],
            semantic_notice: None,
        });
    }

    let query_re = build_query_regex(query)?;
    let scope_prefixes = normalize_scope_prefixes(scope);

    if let Ok(corpus) = scan_corpus(project_path) {
        let fingerprint = fingerprint_corpus(&corpus);
        if let Some(mut output) = try_grep_bm25(project_path, query, scope, top_k, &fingerprint)? {
            if matches!(mode, GrepMode::Hybrid) {
                for hit in &mut output.hits {
                    hit.metadata.insert("degraded".to_string(), json!(true));
                    hit.metadata
                        .insert("degraded_reason".to_string(), json!("hybrid_not_ready"));
                    hit.metadata
                        .insert("semantic_unavailable".to_string(), json!(true));
                }
            }
            return Ok(output);
        }
    }

    let files = list_all_chapter_files(project_path)?;
    let docs = sync_cache_and_snapshot(project_path, files)?;

    let mut hits = search_docs(&docs, &query_re, &scope_prefixes, top_k);

    if scope_prefixes.is_empty() || scope_prefixes.iter().any(|p| p.starts_with(".magic_novel")) {
        hits.extend(grep_assets_keyword(
            project_path,
            &query_re,
            &scope_prefixes,
            top_k,
        )?);

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(top_k as usize);
    }

    if matches!(mode, GrepMode::Hybrid) {
        for hit in &mut hits {
            hit.metadata.insert("degraded".to_string(), json!(true));
            hit.metadata
                .insert("degraded_reason".to_string(), json!("hybrid_not_ready"));
            hit.metadata
                .insert("semantic_unavailable".to_string(), json!(true));
        }
    }

    Ok(GrepOutput {
        hits,
        semantic_notice: None,
    })
}

fn build_query_regex(query: &str) -> Result<Regex, AppError> {
    let escaped = regex::escape(query);

    RegexBuilder::new(&escaped)
        .case_insensitive(true)
        .build()
        .map_err(|err| {
            AppError::invalid_argument(format!("E_TOOL_SCHEMA_INVALID: invalid query: {err}"))
        })
}

fn list_all_chapter_files(project_path: &str) -> Result<Vec<ChapterFileInfo>, AppError> {
    let manuscripts_root = PathBuf::from(project_path).join(MANUSCRIPTS_DIR);
    if !manuscripts_root.exists() {
        return Ok(vec![]);
    }

    let mut out = vec![];

    for volume_id in list_dirs(&manuscripts_root)? {
        let volume_dir = manuscripts_root.join(&volume_id);
        if !volume_dir.join(VOLUME_FILE).exists() {
            continue;
        }

        let files = list_files(&volume_dir, ".json")?;
        for file_name in files {
            if file_name == VOLUME_FILE {
                continue;
            }

            let full_path = volume_dir.join(&file_name);
            let modified_ms = file_modified_millis(&full_path);
            out.push(ChapterFileInfo {
                chapter_path: format!("{volume_id}/{file_name}"),
                full_path,
                modified_ms,
            });
        }
    }

    Ok(out)
}

fn file_modified_millis(path: &Path) -> i64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(system_time_to_millis)
        .unwrap_or(0)
}

fn system_time_to_millis(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}

fn sync_cache_and_snapshot(
    project_path: &str,
    files: Vec<ChapterFileInfo>,
) -> Result<Vec<ChapterDoc>, AppError> {
    let wanted: HashSet<String> = files.iter().map(|f| f.chapter_path.clone()).collect();

    let (to_refresh, to_remove) = {
        let mut cache_map = lock_cache()?;
        let cache = cache_map
            .entry(project_path.to_string())
            .or_insert_with(ProjectChapterCache::default);

        let mut refresh = vec![];
        for info in &files {
            let needs = cache
                .chapters
                .get(&info.chapter_path)
                .map(|c| c.modified_ms != info.modified_ms)
                .unwrap_or(true);

            if needs {
                refresh.push(info.clone());
            }
        }

        let remove: Vec<String> = cache
            .chapters
            .keys()
            .filter(|k| !wanted.contains(*k))
            .cloned()
            .collect();

        (refresh, remove)
    };

    let refreshed = load_refreshed_chapters(&to_refresh);

    {
        let mut cache_map = lock_cache()?;
        let cache = cache_map
            .entry(project_path.to_string())
            .or_insert_with(ProjectChapterCache::default);

        for key in to_remove {
            cache.chapters.remove(&key);
        }

        for (path, value) in refreshed {
            cache.chapters.insert(path, value);
        }

        Ok(snapshot_docs(cache))
    }
}

fn lock_cache(
) -> Result<std::sync::MutexGuard<'static, HashMap<String, ProjectChapterCache>>, AppError> {
    PROJECT_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| AppError::internal("E_TOOL_INTERNAL: grep cache lock poisoned"))
}

fn load_refreshed_chapters(to_refresh: &[ChapterFileInfo]) -> Vec<(String, CachedChapter)> {
    let mut out = vec![];

    for info in to_refresh {
        if let Ok(cached) = load_chapter(info) {
            out.push((info.chapter_path.clone(), cached));
        }
    }

    out
}

fn load_chapter(info: &ChapterFileInfo) -> Result<CachedChapter, AppError> {
    let chapter: Chapter = read_json(&info.full_path)?;
    let text = extract_tiptap_text(&chapter.content);

    Ok(CachedChapter {
        modified_ms: info.modified_ms,
        chapter_id: chapter.id,
        title: chapter.title,
        text: Arc::new(text),
    })
}

fn snapshot_docs(cache: &ProjectChapterCache) -> Vec<ChapterDoc> {
    cache
        .chapters
        .iter()
        .map(|(path, c)| ChapterDoc {
            chapter_path: path.clone(),
            chapter_id: c.chapter_id.clone(),
            title: c.title.clone(),
            text: c.text.clone(),
        })
        .collect()
}

fn search_docs(
    docs: &[ChapterDoc],
    query_re: &Regex,
    scope_prefixes: &[String],
    top_k: u32,
) -> Vec<GrepHit> {
    let mut hits = vec![];

    for doc in docs {
        if !in_scope(&doc.chapter_path, scope_prefixes) {
            continue;
        }

        let Some(matched) = find_keyword(doc.text.as_str(), query_re, MATCH_COUNT_CAP) else {
            continue;
        };

        let snippet = build_snippet(
            doc.text.as_str(),
            matched.start,
            matched.end,
            SNIPPET_CONTEXT_CHARS,
        );

        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("chapter_id".to_string(), json!(doc.chapter_id));
        metadata.insert("title".to_string(), json!(doc.title));
        metadata.insert("match_count".to_string(), json!(matched.count));

        hits.push(GrepHit {
            path: doc.chapter_path.clone(),
            score: score_keyword(&matched),
            snippet,
            metadata,
        });
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    hits.truncate(top_k as usize);
    hits
}
