use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use regex::{Regex, RegexBuilder};
use serde_json::json;

use crate::agent_tools::contracts::{GrepHit, GrepOutput, GrepScope};
use crate::kernel::search::keyword::{build_snippet, find_keyword};
use crate::models::AppError;

use super::build::load_bm25_index;
use super::types::ChunkRecord;
use crate::application::search_usecases::index::io::read_manifest;
use crate::application::search_usecases::index::manager::{EnsureReason, SearchIndexManager};
use crate::application::search_usecases::index::paths::{index_root, manifest_path};
use crate::application::search_usecases::index::scope::{in_scope, normalize_scope_prefixes};

const MATCH_COUNT_CAP: u32 = 64;
const SNIPPET_CONTEXT_CHARS: usize = 60;

#[derive(Clone)]
struct CachedIndex {
    fingerprint: String,
    chunks: Arc<Vec<ChunkRecord>>,
    index: Arc<crate::kernel::search::bm25::Bm25Index>,
}

static INDEX_CACHE: OnceLock<Mutex<HashMap<String, CachedIndex>>> = OnceLock::new();

pub fn try_grep_bm25(
    project_path: &str,
    query: &str,
    scope: Option<&GrepScope>,
    top_k: u32,
    fingerprint: &str,
) -> Result<Option<GrepOutput>, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Some(GrepOutput {
            hits: vec![],
            semantic_notice: None,
        }));
    }

    if let Some(cached) = get_cached(project_path, fingerprint) {
        return Ok(Some(run_query(&cached, query, scope, top_k)?));
    }

    let root = index_root(project_path);
    if let Ok(manifest) = read_manifest(&manifest_path(&root)) {
        if manifest.corpus.fingerprint == fingerprint {
            if let Ok((chunks, index)) = load_bm25_index(project_path) {
                let cached = CachedIndex {
                    fingerprint: fingerprint.to_string(),
                    chunks: Arc::new(chunks),
                    index: Arc::new(index),
                };

                put_cached(project_path, cached.clone());
                return Ok(Some(run_query(&cached, query, scope, top_k)?));
            }
        }
    }

    let _ = SearchIndexManager::global().ensure_index(project_path, EnsureReason::Query);
    Ok(None)
}

#[allow(dead_code)]
pub fn drop_cached(project_path: &str) {
    let cache = INDEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = cache.lock() {
        guard.remove(project_path);
    }
}

fn run_query(
    cached: &CachedIndex,
    query: &str,
    scope: Option<&GrepScope>,
    top_k: u32,
) -> Result<GrepOutput, AppError> {
    let scope_prefixes = normalize_scope_prefixes(scope);
    let scope_mask = build_scope_mask(&cached.chunks, &scope_prefixes);

    let scores = cached.index.score_query(query, Some(&scope_mask));
    let mut scored: Vec<(u32, f64)> = scores.into_iter().collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let cap = (top_k as usize).saturating_mul(4).max(top_k as usize);
    scored.truncate(cap);

    let query_re = build_query_regex(query)?;
    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut hits = vec![];

    for (doc_id, score) in scored {
        let Some(chunk) = cached.chunks.get(doc_id as usize) else {
            continue;
        };

        if !in_scope(&chunk.source_path, &scope_prefixes) {
            continue;
        }

        let path = chunk.source_path.clone();
        if seen_paths.contains(&path) {
            continue;
        }

        let (snippet, match_count) = build_hit_snippet(&chunk.text, &query_re);
        let metadata = build_hit_metadata(chunk, match_count);

        hits.push(GrepHit {
            path: path.clone(),
            score,
            snippet,
            metadata,
        });

        seen_paths.insert(path);
        if hits.len() >= top_k as usize {
            break;
        }
    }

    Ok(GrepOutput {
        hits,
        semantic_notice: None,
    })
}

fn build_hit_metadata(
    chunk: &ChunkRecord,
    match_count: Option<u32>,
) -> HashMap<String, serde_json::Value> {
    let mut metadata = HashMap::new();
    metadata.insert("engine".to_string(), json!("bm25"));
    metadata.insert("source_kind".to_string(), json!(chunk.source_kind));
    metadata.insert("title".to_string(), json!(chunk.title));
    metadata.insert("chunk_id".to_string(), json!(chunk.chunk_id));

    if let Some(count) = match_count {
        metadata.insert("match_count".to_string(), json!(count));
    }

    metadata
}

fn build_scope_mask(chunks: &[ChunkRecord], scope_prefixes: &[String]) -> Vec<bool> {
    if scope_prefixes.is_empty() {
        return vec![true; chunks.len()];
    }

    chunks
        .iter()
        .map(|chunk| in_scope(&chunk.source_path, scope_prefixes))
        .collect()
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

fn build_hit_snippet(text: &str, query_re: &Regex) -> (String, Option<u32>) {
    if let Some(matched) = find_keyword(text, query_re, MATCH_COUNT_CAP) {
        let snippet = build_snippet(text, matched.start, matched.end, SNIPPET_CONTEXT_CHARS);
        return (snippet, Some(matched.count));
    }

    let trimmed = text.trim();
    if trimmed.chars().count() > 200 {
        let short = trimmed.chars().take(200).collect::<String>() + "...";
        return (short, None);
    }

    (trimmed.to_string(), None)
}

fn get_cached(project_path: &str, fingerprint: &str) -> Option<CachedIndex> {
    let cache = INDEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let guard = cache.lock().ok()?;
    let entry = guard.get(project_path)?;

    if entry.fingerprint == fingerprint {
        return Some(entry.clone());
    }

    None
}

fn put_cached(project_path: &str, cached: CachedIndex) {
    let cache = INDEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(project_path.to_string(), cached);
    }
}
