use std::path::{Component, PathBuf};

use crate::knowledge::types::KNOWLEDGE_PROPOSAL_INVALID;
use crate::models::AppError;

pub(super) fn normalize_path(input: &str) -> String {
    let mut p = input.trim().replace('\\', "/");
    while p.starts_with("./") {
        p = p.trim_start_matches("./").to_string();
    }
    while p.contains("//") {
        p = p.replace("//", "/");
    }
    p.trim_matches('/').to_string()
}

pub(super) fn ensure_safe_relative_path(rel: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(rel);
    if p.is_absolute() {
        return Err(AppError::invalid_argument(format!(
            "{KNOWLEDGE_PROPOSAL_INVALID}: target_ref must be a relative path"
        )));
    }

    for c in p.components() {
        match c {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::invalid_argument(format!(
                    "{KNOWLEDGE_PROPOSAL_INVALID}: unsafe target_ref"
                )));
            }
            _ => {}
        }
    }

    Ok(p)
}

pub(super) fn slugify_locator(locator: &str) -> String {
    normalize_path(locator)
        .trim_end_matches(".json")
        .trim_end_matches(".md")
        .replace(&['/', ' ', ':'][..], "_")
        .replace("__", "_")
        .trim_matches('_')
        .to_string()
}

pub(super) fn infer_chapter_locator_from_write_paths(write_paths: &[String]) -> Option<String> {
    for p in write_paths {
        let p = normalize_path(p);
        if p.starts_with("manuscripts/") && p.ends_with(".json") && !p.ends_with("/volume.json") {
            return Some(p.trim_start_matches("manuscripts/").to_string());
        }
    }
    None
}

