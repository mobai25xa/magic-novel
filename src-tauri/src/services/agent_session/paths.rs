use std::path::{Path, PathBuf};

pub const AGENT_SESSION_SCHEMA_VERSION: i32 = 1;
pub const MAGIC_NOVEL_DIR: &str = "magic_novel";
pub const AI_DIR: &str = "ai";
pub const SESSIONS_DIR: &str = "sessions";
pub const INDEX_FILE: &str = "index.json";
pub const STREAM_SUFFIX: &str = ".jsonl";
pub const SETTINGS_SUFFIX: &str = ".settings.json";

pub fn sessions_root(project_path: &Path) -> PathBuf {
    project_path
        .join(MAGIC_NOVEL_DIR)
        .join(AI_DIR)
        .join(SESSIONS_DIR)
}

pub fn session_stream_path(project_path: &Path, session_id: &str) -> PathBuf {
    sessions_root(project_path).join(format!("{session_id}{STREAM_SUFFIX}"))
}

pub fn session_settings_path(project_path: &Path, session_id: &str) -> PathBuf {
    sessions_root(project_path).join(format!("{session_id}{SETTINGS_SUFFIX}"))
}

pub fn session_index_path(project_path: &Path) -> PathBuf {
    sessions_root(project_path).join(INDEX_FILE)
}
