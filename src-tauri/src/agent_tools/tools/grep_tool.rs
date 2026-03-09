use serde_json::json;

use crate::agent_tools::contracts::{GrepInput, GrepMode, GrepOutput, GrepSemanticNotice};
use crate::application::search_usecases::{grep_hybrid, grep_keyword, grep_semantic};
use crate::models::AppError;
use crate::services::load_openai_search_settings;

const SEMANTIC_UNAVAILABLE_MESSAGE: &str =
    "Embedding semantic retrieval unavailable; keyword search used";

pub fn run(input: GrepInput, _call_id: &str) -> Result<serde_json::Value, AppError> {
    validate_grep_input(&input)?;

    let requested_mode = input.mode.clone();
    let effective_mode = resolve_effective_mode(&requested_mode)?;
    let query = input.query.trim();

    let mut out = match effective_mode {
        GrepMode::Keyword => grep_keyword(
            &input.project_path,
            query,
            input.scope.as_ref(),
            input.top_k,
        )?,
        GrepMode::Semantic => grep_semantic(
            &input.project_path,
            query,
            input.scope.as_ref(),
            input.top_k,
        )?,
        GrepMode::Hybrid => grep_hybrid(
            &input.project_path,
            query,
            input.scope.as_ref(),
            input.top_k,
        )?,
    };

    if !matches!(requested_mode, GrepMode::Keyword) && matches!(effective_mode, GrepMode::Keyword) {
        apply_forced_keyword_notice(&mut out, "embedding_disabled");
    }

    Ok(json!(out))
}

fn resolve_effective_mode(requested_mode: &GrepMode) -> Result<GrepMode, AppError> {
    if matches!(requested_mode, GrepMode::Keyword) {
        return Ok(GrepMode::Keyword);
    }

    let settings = load_openai_search_settings()?;
    if settings.openai_embedding_enabled {
        return Ok(requested_mode.clone());
    }

    Ok(GrepMode::Keyword)
}

fn apply_forced_keyword_notice(out: &mut GrepOutput, reason: &str) {
    for hit in &mut out.hits {
        hit.metadata.insert("degraded".to_string(), json!(true));
        hit.metadata
            .insert("degraded_reason".to_string(), json!(reason));
        hit.metadata
            .insert("semantic_unavailable".to_string(), json!(true));
    }

    out.semantic_notice = Some(GrepSemanticNotice {
        semantic_retrieval_available: false,
        reason: Some(reason.to_string()),
        message: Some(SEMANTIC_UNAVAILABLE_MESSAGE.to_string()),
    });
}

fn validate_grep_input(input: &GrepInput) -> Result<(), AppError> {
    if input.project_path.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: project_path is required",
        ));
    }

    if input.query.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "E_TOOL_SCHEMA_INVALID: query is required",
        ));
    }

    match input.mode {
        GrepMode::Keyword | GrepMode::Semantic | GrepMode::Hybrid => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::OpenAiSearchSettings;
    use std::path::PathBuf;

    fn unique_home() -> PathBuf {
        std::env::temp_dir().join(format!("magic_grep_tool_home_{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn test_semantic_falls_back_when_embedding_disabled() {
        let home = unique_home();
        std::fs::create_dir_all(home.join(".magic")).unwrap();
        std::env::set_var("HOME", &home);
        std::env::set_var("USERPROFILE", &home);

        let settings = OpenAiSearchSettings {
            openai_embedding_enabled: false,
            ..OpenAiSearchSettings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        std::fs::write(home.join(".magic").join("setting.json"), json).unwrap();

        let input = GrepInput {
            project_path: std::env::temp_dir().to_string_lossy().to_string(),
            query: "test".to_string(),
            mode: GrepMode::Semantic,
            scope: None,
            top_k: 3,
        };

        let out = run(input, "call_1").unwrap();
        let notice = out.get("semantic_notice").unwrap();
        assert_eq!(notice["semantic_retrieval_available"], false);
        assert_eq!(notice["reason"], "embedding_disabled");
    }

    #[test]
    fn test_hybrid_falls_back_when_embedding_disabled() {
        let home = unique_home();
        std::fs::create_dir_all(home.join(".magic")).unwrap();
        std::env::set_var("HOME", &home);
        std::env::set_var("USERPROFILE", &home);

        let settings = OpenAiSearchSettings {
            openai_embedding_enabled: false,
            ..OpenAiSearchSettings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        std::fs::write(home.join(".magic").join("setting.json"), json).unwrap();

        let input = GrepInput {
            project_path: std::env::temp_dir().to_string_lossy().to_string(),
            query: "test".to_string(),
            mode: GrepMode::Hybrid,
            scope: None,
            top_k: 3,
        };

        let out = run(input, "call_2").unwrap();
        let notice = out.get("semantic_notice").unwrap();
        assert_eq!(notice["semantic_retrieval_available"], false);
        assert_eq!(notice["reason"], "embedding_disabled");
    }
}
