use serde::{Deserialize, Serialize};

use crate::agent_tools::contracts::GrepScope;
use crate::agent_tools::tools::r#ref::normalize_project_relative_path;
use crate::application::search_usecases::{grep_hybrid, grep_keyword, grep_semantic};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSearchCorpus {
    Draft,
    Knowledge,
    All,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSearchMode {
    Keyword,
    Semantic,
    Hybrid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSearchScopePaths {
    pub paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSearchArgs {
    pub query: String,
    pub corpus: Option<ContextSearchCorpus>,
    pub mode: Option<ContextSearchMode>,
    pub top_k: Option<u32>,
    pub scope: Option<ContextSearchScopePaths>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextSearchHit {
    #[serde(rename = "ref")]
    pub ref_: String,
    pub score: f64,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextSearchOutput {
    pub hits: Vec<ContextSearchHit>,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContextSearchRun {
    pub output: ContextSearchOutput,
    pub read_set: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ContextSearchError {
    pub code: &'static str,
    pub message: String,
}

pub fn run_context_search(
    project_path: &str,
    args: ContextSearchArgs,
) -> Result<ContextSearchRun, ContextSearchError> {
    let project_path = project_path.trim();
    if project_path.is_empty() {
        return Err(ContextSearchError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "missing project_path".to_string(),
        });
    }

    let query = args.query.trim();
    if query.is_empty() {
        return Err(ContextSearchError {
            code: "E_TOOL_SCHEMA_INVALID",
            message: "query must be a non-empty string".to_string(),
        });
    }

    let corpus = args.corpus.unwrap_or(ContextSearchCorpus::All);
    let requested_mode = args.mode.unwrap_or(ContextSearchMode::Keyword);
    let top_k = args.top_k.unwrap_or(10).clamp(1, 100);
    let scope = build_scope(args.scope.as_ref())?;

    let grep_out = match requested_mode {
        ContextSearchMode::Keyword => grep_keyword(project_path, query, scope.as_ref(), top_k),
        ContextSearchMode::Semantic => grep_semantic(project_path, query, scope.as_ref(), top_k),
        ContextSearchMode::Hybrid => grep_hybrid(project_path, query, scope.as_ref(), top_k),
    }
    .map_err(|_| ContextSearchError {
        code: "E_INTERNAL",
        message: "search failed".to_string(),
    })?;

    let mut degraded = false;
    let mut degraded_reason: Option<String> = None;
    let mut effective_mode = match requested_mode {
        ContextSearchMode::Keyword => "keyword".to_string(),
        ContextSearchMode::Semantic => "semantic".to_string(),
        ContextSearchMode::Hybrid => "hybrid".to_string(),
    };

    if let Some(notice) = &grep_out.semantic_notice {
        if !notice.semantic_retrieval_available {
            degraded = true;
            degraded_reason = notice
                .reason
                .clone()
                .or_else(|| Some("embedding_unavailable".to_string()));
            effective_mode = "keyword".to_string();
        }
    }

    let mut hits = Vec::new();
    for hit in grep_out.hits {
        if !matches_corpus(&hit.path, corpus) {
            continue;
        }

        let (ref_, path) = to_ref_and_path(&hit.path)?;
        hits.push(ContextSearchHit {
            ref_,
            score: hit.score,
            snippet: limit_snippet(&hit.snippet, 400),
            path: Some(path),
            metadata: Some(serde_json::Value::Object(
                hit.metadata.into_iter().collect(),
            )),
        });

        if hits.len() >= top_k as usize {
            break;
        }
    }

    Ok(ContextSearchRun {
        output: ContextSearchOutput {
            hits,
            mode: effective_mode,
            degraded: degraded.then_some(true),
            degraded_reason,
        },
        read_set: None,
    })
}

fn build_scope(
    scope: Option<&ContextSearchScopePaths>,
) -> Result<Option<GrepScope>, ContextSearchError> {
    let Some(scope) = scope else {
        return Ok(None);
    };
    let Some(paths) = scope.paths.as_ref() else {
        return Ok(None);
    };

    let mut out = Vec::new();
    for raw in paths {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }

        let normalized =
            normalize_project_relative_path(raw, false).map_err(|err| ContextSearchError {
                code: "E_TOOL_SCHEMA_INVALID",
                message: err.message,
            })?;
        if !out.contains(&normalized) {
            out.push(normalized);
        }
    }

    if out.is_empty() {
        Ok(None)
    } else {
        Ok(Some(GrepScope { paths: out }))
    }
}

fn matches_corpus(path: &str, corpus: ContextSearchCorpus) -> bool {
    match corpus {
        ContextSearchCorpus::All => true,
        ContextSearchCorpus::Draft => !path.starts_with(".magic_novel/"),
        ContextSearchCorpus::Knowledge => path.starts_with(".magic_novel/"),
    }
}

fn to_ref_and_path(path: &str) -> Result<(String, String), ContextSearchError> {
    let normalized =
        normalize_project_relative_path(path, false).map_err(|err| ContextSearchError {
            code: "E_INTERNAL",
            message: err.message,
        })?;

    if normalized.starts_with(".magic_novel/") {
        let ref_ = format!("knowledge:{normalized}");
        return Ok((ref_, normalized));
    }

    let project_rel = if normalized.starts_with("manuscripts/") {
        normalized
    } else {
        format!("manuscripts/{normalized}")
    };

    Ok((format!("chapter:{project_rel}"), project_rel))
}

fn limit_snippet(snippet: &str, max_chars: usize) -> String {
    let trimmed = snippet.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let head = trimmed.chars().take(max_chars).collect::<String>();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Chapter, VolumeMetadata};
    use crate::services;
    use tempfile::tempdir;

    fn setup_project() -> (tempfile::TempDir, String) {
        let dir = tempdir().expect("temp");
        let project = dir.path().to_path_buf();
        let vol_dir = project.join("manuscripts").join("vol_1");
        services::ensure_dir(&vol_dir).expect("dir");

        let mut vol = VolumeMetadata::new("Vol 1".to_string());
        vol.volume_id = "vol_1".to_string();
        services::write_json(&vol_dir.join("volume.json"), &vol).expect("volume");

        let mut ch1 = Chapter::new("Apple".to_string());
        ch1.id = "ch_apple".to_string();
        ch1.content = serde_json::json!("apple orange");
        services::write_json(&vol_dir.join("ch_apple.json"), &ch1).expect("chapter1");

        let mut ch2 = Chapter::new("Banana".to_string());
        ch2.id = "ch_banana".to_string();
        ch2.content = serde_json::json!("banana pear");
        services::write_json(&vol_dir.join("ch_banana.json"), &ch2).expect("chapter2");

        (dir, project.to_string_lossy().to_string())
    }

    #[test]
    fn empty_query_is_invalid() {
        let (_dir, project_path) = setup_project();
        let err = run_context_search(
            &project_path,
            ContextSearchArgs {
                query: "   ".to_string(),
                corpus: None,
                mode: None,
                top_k: None,
                scope: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "E_TOOL_SCHEMA_INVALID");
    }

    #[test]
    fn keyword_search_returns_chapter_refs() {
        let (_dir, project_path) = setup_project();
        let run = run_context_search(
            &project_path,
            ContextSearchArgs {
                query: "apple".to_string(),
                corpus: Some(ContextSearchCorpus::Draft),
                mode: Some(ContextSearchMode::Keyword),
                top_k: Some(10),
                scope: None,
            },
        )
        .expect("run");

        assert!(!run.output.hits.is_empty());
        assert!(run.output.hits[0].ref_.starts_with("chapter:manuscripts/"));
    }

    #[test]
    fn scope_paths_filters_hits() {
        let (_dir, project_path) = setup_project();
        let run = run_context_search(
            &project_path,
            ContextSearchArgs {
                query: "banana".to_string(),
                corpus: Some(ContextSearchCorpus::Draft),
                mode: Some(ContextSearchMode::Keyword),
                top_k: Some(10),
                scope: Some(ContextSearchScopePaths {
                    paths: Some(vec!["manuscripts/vol_1/ch_banana.json".to_string()]),
                }),
            },
        )
        .expect("run");

        assert_eq!(run.output.hits.len(), 1);
        assert!(run.output.hits[0]
            .ref_
            .contains("manuscripts/vol_1/ch_banana.json"));
    }
}
