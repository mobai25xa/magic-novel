use std::path::{Path, PathBuf};

use crate::models::AppError;

pub const KNOWLEDGE_ROOT_PRIMARY: &str = ".magic_novel";
pub const KNOWLEDGE_ROOT_FALLBACK: &str = "magic_assets";

pub fn knowledge_root_exists(project_path: &Path) -> bool {
    project_path.join(KNOWLEDGE_ROOT_PRIMARY).exists()
        || project_path.join(KNOWLEDGE_ROOT_FALLBACK).exists()
}

pub fn resolve_knowledge_root_for_read(project_path: &Path) -> PathBuf {
    let primary = project_path.join(KNOWLEDGE_ROOT_PRIMARY);
    if primary.exists() {
        return primary;
    }

    let fallback = project_path.join(KNOWLEDGE_ROOT_FALLBACK);
    if fallback.exists() {
        return fallback;
    }

    primary
}

pub fn resolve_knowledge_root_for_write(project_path: &Path) -> Result<PathBuf, AppError> {
    let primary = project_path.join(KNOWLEDGE_ROOT_PRIMARY);
    if primary.exists() {
        return Ok(primary);
    }

    let fallback = project_path.join(KNOWLEDGE_ROOT_FALLBACK);
    if fallback.exists() {
        return Ok(fallback);
    }

    std::fs::create_dir_all(&primary)?;
    Ok(primary)
}

/// Map a virtual `.magic_novel/...` path used by tools/UI into the physical knowledge root.
///
/// Compatibility: if `.magic_novel/` doesn't exist, falls back to `magic_assets/`.
#[allow(dead_code)]
pub fn map_virtual_magic_novel_path(project_path: &Path, virtual_path: &str) -> PathBuf {
    let v = virtual_path.trim().replace('\\', "/");
    let v = v.trim_start_matches("./").trim().trim_end_matches('/');

    let rel = v
        .strip_prefix(".magic_novel")
        .unwrap_or(v)
        .trim_start_matches('/');

    resolve_knowledge_root_for_read(project_path).join(rel)
}
