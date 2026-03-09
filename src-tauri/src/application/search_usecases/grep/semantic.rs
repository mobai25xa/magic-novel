use std::collections::HashMap;

use regex::{Regex, RegexBuilder};
use serde_json::json;

use crate::agent_tools::contracts::{GrepHit, GrepMode, GrepOutput, GrepScope};
use crate::kernel::search::keyword::{build_snippet, find_keyword};
use crate::models::{AppError, ErrorCode};

use super::keyword::grep_keyword_with_mode;

use super::super::vector::{
    embed_query, ensure_vector_index, expand_with_neighbors, query_vector_topn,
};
use crate::application::search_usecases::index::scope::{in_scope, normalize_scope_prefixes};

const MATCH_COUNT_CAP: u32 = 64;
const SNIPPET_CONTEXT_CHARS: usize = 60;

pub fn grep_semantic(
    project_path: &str,
    query: &str,
    scope: Option<&GrepScope>,
    top_k: u32,
) -> Result<GrepOutput, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(GrepOutput {
            hits: vec![],
            semantic_notice: None,
        });
    }

    match grep_semantic_inner(project_path, query, scope, top_k) {
        Ok(out) => Ok(out),
        Err(err) if err.recoverable.unwrap_or(false) => {
            Ok(degraded_semantic(project_path, query, scope, top_k, err))
        }
        Err(err) => Err(err),
    }
}

fn grep_semantic_inner(
    project_path: &str,
    query: &str,
    scope: Option<&GrepScope>,
    top_k: u32,
) -> Result<GrepOutput, AppError> {
    let loaded = ensure_vector_index(project_path)?;
    let scope_prefixes = normalize_scope_prefixes(scope);
    let scope_mask = build_scope_mask(&loaded.chunks, &scope_prefixes);

    let query_vector = embed_query(project_path, query)?;
    let vector_top = query_vector_topn(
        &query_vector,
        &loaded,
        Some(&scope_mask),
        (top_k as usize).saturating_mul(5).max(top_k as usize),
    )?;

    let query_re = build_query_regex(query)?;
    let mut hits = Vec::new();

    for (chunk_idx, score) in vector_top {
        let Some(chunk) = loaded.chunks.get(chunk_idx) else {
            continue;
        };

        if !in_scope(&chunk.source_path, &scope_prefixes) {
            continue;
        }

        let expanded = expand_with_neighbors(&loaded.chunks, chunk_idx);
        let snippet_source = if expanded.trim().is_empty() {
            chunk.text.as_str()
        } else {
            expanded.as_str()
        };

        let (snippet, match_count) = build_hit_snippet(snippet_source, &query_re);

        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("engine".to_string(), json!("vector"));
        metadata.insert("source_kind".to_string(), json!(chunk.source_kind));
        metadata.insert("title".to_string(), json!(chunk.title));
        metadata.insert("chunk_id".to_string(), json!(chunk.chunk_id));
        if let Some(count) = match_count {
            metadata.insert("match_count".to_string(), json!(count));
        }

        hits.push(GrepHit {
            path: chunk.source_path.clone(),
            score,
            snippet,
            metadata,
        });

        if hits.len() >= top_k as usize {
            break;
        }
    }

    Ok(GrepOutput {
        hits,
        semantic_notice: None,
    })
}

fn degraded_semantic(
    project_path: &str,
    query: &str,
    scope: Option<&GrepScope>,
    top_k: u32,
    err: AppError,
) -> GrepOutput {
    let reason = err
        .details
        .as_ref()
        .and_then(|value| value.get("reason"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| "embedding_unavailable".to_string());

    let mut out = grep_keyword_with_mode(project_path, query, scope, top_k, GrepMode::Keyword)
        .unwrap_or_else(|_| GrepOutput {
            hits: vec![],
            semantic_notice: None,
        });

    for hit in &mut out.hits {
        hit.metadata.insert("degraded".to_string(), json!(true));
        hit.metadata
            .insert("degraded_reason".to_string(), json!(reason));
        hit.metadata
            .insert("semantic_unavailable".to_string(), json!(true));
        hit.metadata.insert(
            "semantic_unavailable_code".to_string(),
            json!(extract_error_code(&err)),
        );
        hit.metadata.insert(
            "semantic_unavailable_message".to_string(),
            json!(err.message.clone()),
        );
        hit.metadata.insert("engine".to_string(), json!("keyword"));
    }

    out.semantic_notice = Some(crate::agent_tools::contracts::GrepSemanticNotice {
        semantic_retrieval_available: false,
        reason: Some(reason),
        message: Some("Embedding semantic retrieval unavailable; keyword search used".to_string()),
    });

    out
}

fn extract_error_code(err: &AppError) -> String {
    err.details
        .as_ref()
        .and_then(|value| value.get("code"))
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| match err.code {
            ErrorCode::InvalidArgument => "E_AI_SETTINGS_EMBEDDING_UNAVAILABLE",
            _ => "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR",
        })
        .to_string()
}

fn build_scope_mask(
    chunks: &[crate::application::search_usecases::index::types::ChunkRecord],
    scope_prefixes: &[String],
) -> Vec<bool> {
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
