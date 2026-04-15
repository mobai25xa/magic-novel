use std::path::{Path, PathBuf};

use crate::models::{AppError, ErrorCode};

pub(crate) fn normalize_review_target_ref(target_ref: &str) -> Result<String, AppError> {
    let raw = target_ref.trim();
    if raw.is_empty() {
        return Err(AppError::invalid_argument("empty target_ref"));
    }

    if let Ok(parsed) = crate::agent_tools::tools::r#ref::parse_tool_ref(raw) {
        if parsed.path.is_empty() {
            return Err(AppError::invalid_argument("empty target_ref"));
        }
        return Ok(parsed.path);
    }

    let normalized = raw.replace('\\', "/");
    if normalized.starts_with('/') || normalized.split('/').any(|seg| seg == "..") {
        return Err(AppError::invalid_argument(format!(
            "invalid target_ref: {normalized}"
        )));
    }

    if normalized.contains(':') {
        return Err(AppError::invalid_argument(format!(
            "invalid target_ref: {normalized}"
        )));
    }

    Ok(normalized)
}

pub(crate) fn resolve_review_target_path(
    project_path: &Path,
    target_ref: &str,
) -> Result<(String, PathBuf), AppError> {
    let normalized = normalize_review_target_ref(target_ref)?;
    let candidates = [
        PathBuf::from(project_path).join(&normalized),
        PathBuf::from(project_path)
            .join("manuscripts")
            .join(&normalized),
    ];

    let full = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| AppError {
            code: ErrorCode::NotFound,
            message: format!("review target not found: {normalized}"),
            details: Some(serde_json::json!({
                "code": "REVIEW_INPUT_MISSING",
                "target_ref": normalized,
            })),
            recoverable: Some(true),
        })?;

    Ok((normalized, full))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_review_target_ref_accepts_full_chapter_ref() {
        let normalized =
            normalize_review_target_ref("chapter:manuscripts/vol_1/ch_1.json").expect("ref");
        assert_eq!(normalized, "manuscripts/vol_1/ch_1.json");
    }

    #[test]
    fn normalize_review_target_ref_accepts_shorthand_path() {
        let normalized = normalize_review_target_ref("vol_1/ch_1.json").expect("path");
        assert_eq!(normalized, "vol_1/ch_1.json");
    }

    #[test]
    fn normalize_review_target_ref_rejects_unknown_ref_kind() {
        let err = normalize_review_target_ref("mystery:foo/bar.json").expect_err("invalid ref");
        assert!(err.message.contains("invalid target_ref"));
    }
}
