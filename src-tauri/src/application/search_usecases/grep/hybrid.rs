use std::collections::HashMap;

use serde_json::json;

use crate::agent_tools::contracts::{GrepHit, GrepOutput, GrepScope};
use crate::models::AppError;

use super::keyword::grep_keyword_with_mode;
use super::semantic::grep_semantic;
use crate::application::search_usecases::vector::{expand_with_neighbors, merge_rrf};

const PER_PATH_LIMIT: usize = 2;

pub fn grep_hybrid(
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

    let keyword_out = grep_keyword_with_mode(
        project_path,
        query,
        scope,
        top_k.saturating_mul(5).max(top_k),
        crate::agent_tools::contracts::GrepMode::Keyword,
    )?;

    let semantic_out = match grep_semantic(
        project_path,
        query,
        scope,
        top_k.saturating_mul(5).max(top_k),
    ) {
        Ok(out) => out,
        Err(_) => {
            return Ok(degraded_keyword(
                keyword_out,
                top_k,
                Some("vectors_unavailable"),
            ))
        }
    };

    if semantic_out.hits.iter().any(|hit| {
        hit.metadata
            .get("degraded")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }) {
        let reason = semantic_out.hits.iter().find_map(|hit| {
            hit.metadata
                .get("degraded_reason")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });

        return Ok(degraded_keyword(keyword_out, top_k, reason.as_deref()));
    }

    let mut merged = merge_rrf(&keyword_out.hits, &semantic_out.hits);
    enrich_neighbor_snippets(&mut merged, project_path);
    apply_path_diversity(&mut merged, top_k as usize);

    Ok(GrepOutput {
        hits: merged,
        semantic_notice: None,
    })
}

fn degraded_keyword(keyword_out: GrepOutput, top_k: u32, reason: Option<&str>) -> GrepOutput {
    let mut degraded_hits = keyword_out.hits;
    let normalized_reason = reason
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("vectors_unavailable");

    for hit in &mut degraded_hits {
        hit.metadata.insert("degraded".to_string(), json!(true));
        hit.metadata
            .insert("degraded_reason".to_string(), json!(normalized_reason));
        hit.metadata
            .insert("semantic_unavailable".to_string(), json!(true));
        hit.metadata.insert("engine".to_string(), json!("bm25"));
    }

    degraded_hits.truncate(top_k as usize);
    GrepOutput {
        hits: degraded_hits,
        semantic_notice: Some(crate::agent_tools::contracts::GrepSemanticNotice {
            semantic_retrieval_available: false,
            reason: Some(normalized_reason.to_string()),
            message: Some(
                "Embedding semantic retrieval unavailable; keyword search used".to_string(),
            ),
        }),
    }
}

fn enrich_neighbor_snippets(hits: &mut [GrepHit], project_path: &str) {
    let Ok((chunks, _)) =
        crate::application::search_usecases::bm25::build::load_bm25_index(project_path)
    else {
        return;
    };

    let mut chunk_index = HashMap::new();
    for (idx, chunk) in chunks.iter().enumerate() {
        chunk_index.insert(chunk.chunk_id.clone(), idx);
    }

    for hit in hits.iter_mut() {
        let Some(chunk_id) = hit.metadata.get("chunk_id").and_then(|v| v.as_str()) else {
            continue;
        };

        let Some(idx) = chunk_index.get(chunk_id).copied() else {
            continue;
        };

        let expanded = expand_with_neighbors(&chunks, idx);
        if !expanded.trim().is_empty() {
            hit.snippet = expanded;
            hit.metadata
                .insert("neighbor_expanded".to_string(), json!(true));
        }
    }
}

fn apply_path_diversity(hits: &mut Vec<GrepHit>, top_k: usize) {
    let mut limited = Vec::with_capacity(top_k);
    let mut per_path_counts: HashMap<String, usize> = HashMap::new();

    for hit in hits.iter() {
        let count = per_path_counts.entry(hit.path.clone()).or_insert(0);
        if *count >= PER_PATH_LIMIT {
            continue;
        }

        limited.push(hit.clone());
        *count += 1;

        if limited.len() >= top_k {
            break;
        }
    }

    *hits = limited;
}
