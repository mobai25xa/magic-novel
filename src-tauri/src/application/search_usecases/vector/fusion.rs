use std::cmp::Ordering;
use std::collections::HashMap;

use serde_json::json;

use crate::agent_tools::contracts::GrepHit;

const RRF_K: f64 = 60.0;

#[derive(Clone)]
struct RankedHit {
    hit: GrepHit,
    rank_keyword: Option<usize>,
    rank_vector: Option<usize>,
    rrf_score: f64,
}

pub fn merge_rrf(keyword_hits: &[GrepHit], semantic_hits: &[GrepHit]) -> Vec<GrepHit> {
    let mut by_path: HashMap<String, RankedHit> = HashMap::new();

    for (idx, hit) in keyword_hits.iter().enumerate() {
        let rank = idx + 1;
        let entry = by_path
            .entry(hit.path.clone())
            .or_insert_with(|| RankedHit {
                hit: hit.clone(),
                rank_keyword: None,
                rank_vector: None,
                rrf_score: 0.0,
            });

        entry.rank_keyword = Some(rank);
        entry.rrf_score += 1.0 / (RRF_K + rank as f64);
    }

    for (idx, hit) in semantic_hits.iter().enumerate() {
        let rank = idx + 1;
        let entry = by_path
            .entry(hit.path.clone())
            .or_insert_with(|| RankedHit {
                hit: hit.clone(),
                rank_keyword: None,
                rank_vector: None,
                rrf_score: 0.0,
            });

        entry.rank_vector = Some(rank);
        entry.rrf_score += 1.0 / (RRF_K + rank as f64);

        if entry.hit.snippet.trim().is_empty() {
            entry.hit.snippet = hit.snippet.clone();
        }
        if hit.score > entry.hit.score {
            entry.hit.score = hit.score;
        }
    }

    let mut ranked: Vec<RankedHit> = by_path.into_values().collect();
    ranked.sort_by(|a, b| {
        b.rrf_score
            .partial_cmp(&a.rrf_score)
            .unwrap_or(Ordering::Equal)
    });

    ranked
        .into_iter()
        .map(|ranked_hit| {
            let mut hit = ranked_hit.hit;
            hit.score = ranked_hit.rrf_score;
            hit.metadata.insert("engine".to_string(), json!("hybrid"));
            hit.metadata
                .insert("rrf_score".to_string(), json!(ranked_hit.rrf_score));
            hit.metadata.insert(
                "rank_keyword".to_string(),
                ranked_hit
                    .rank_keyword
                    .map_or(serde_json::Value::Null, |v| json!(v)),
            );
            hit.metadata.insert(
                "rank_vector".to_string(),
                ranked_hit
                    .rank_vector
                    .map_or(serde_json::Value::Null, |v| json!(v)),
            );
            hit
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::merge_rrf;
    use crate::agent_tools::contracts::GrepHit;
    use std::collections::HashMap;

    fn make_hit(path: &str, score: f64) -> GrepHit {
        GrepHit {
            path: path.to_string(),
            score,
            snippet: "snippet".to_string(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn rrf_prefers_path_appearing_in_both_lists() {
        let keyword = vec![make_hit("a", 10.0), make_hit("b", 9.0)];
        let vector = vec![make_hit("b", 1.0), make_hit("c", 0.9)];

        let merged = merge_rrf(&keyword, &vector);
        assert_eq!(merged.first().map(|hit| hit.path.as_str()), Some("b"));
    }
}
