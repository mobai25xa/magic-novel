use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

use crate::models::AppError;

use super::io::read_manifest;
use super::paths::{index_lock_path, index_root, vecs_f32_path};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexState {
    Missing,
    Building,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildProgress {
    pub stage: String,
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchIndexStatus {
    pub state: IndexState,
    pub fingerprint_current: Option<String>,
    pub fingerprint_indexed: Option<String>,
    pub bm25_ready: bool,
    pub vectors_ready: bool,
    pub progress: Option<BuildProgress>,
    pub last_error: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnsureReason {
    Query,
    Warmup,
    Rebuild,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnsureResult {
    Started,
    AlreadyBuilding,
    Ready,
    #[allow(dead_code)]
    Failed,
    Cancelled,
}

#[derive(Debug)]
struct ProjectIndexState {
    status: SearchIndexStatus,
    cancel: Arc<AtomicBool>,
}

impl Default for ProjectIndexState {
    fn default() -> Self {
        Self {
            status: SearchIndexStatus {
                state: IndexState::Missing,
                fingerprint_current: None,
                fingerprint_indexed: None,
                bm25_ready: false,
                vectors_ready: false,
                progress: None,
                last_error: None,
                updated_at: chrono::Utc::now().timestamp_millis(),
            },
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct SearchIndexManager {
    states: Mutex<HashMap<String, ProjectIndexState>>,
}

impl SearchIndexManager {
    fn new() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
        }
    }

    pub fn global() -> &'static SearchIndexManager {
        static INSTANCE: OnceLock<SearchIndexManager> = OnceLock::new();
        INSTANCE.get_or_init(SearchIndexManager::new)
    }

    pub fn ensure_index(
        &self,
        project_path: &str,
        reason: EnsureReason,
    ) -> Result<EnsureResult, AppError> {
        let project = project_path.trim();
        if project.is_empty() {
            return Err(AppError::invalid_argument("project_path is required"));
        }

        super::helpers::ensure_index_impl(self, project, reason)
    }

    pub fn status(&self, project_path: &str) -> SearchIndexStatus {
        let project = project_path.trim();
        if project.is_empty() {
            return SearchIndexStatus {
                state: IndexState::Missing,
                fingerprint_current: None,
                fingerprint_indexed: None,
                bm25_ready: false,
                vectors_ready: false,
                progress: None,
                last_error: Some("project_path is required".to_string()),
                updated_at: chrono::Utc::now().timestamp_millis(),
            };
        }

        super::helpers::snapshot_status(self, project)
    }

    pub fn rebuild(&self, project_path: &str, force: bool) -> Result<EnsureResult, AppError> {
        let reason = if force {
            EnsureReason::Rebuild
        } else {
            EnsureReason::Warmup
        };
        self.ensure_index(project_path, reason)
    }

    pub fn cancel(&self, project_path: &str) -> bool {
        let project = project_path.trim();
        if project.is_empty() {
            return false;
        }

        let Ok(mut guard) = self.states.lock() else {
            return false;
        };

        let Some(state) = guard.get_mut(project) else {
            return false;
        };

        if !matches!(state.status.state, IndexState::Building) {
            return false;
        }

        state.cancel.store(true, Ordering::Relaxed);
        state.status.state = IndexState::Failed;
        state.status.progress = None;
        state.status.last_error = Some("cancelled".to_string());
        state.status.updated_at = chrono::Utc::now().timestamp_millis();
        true
    }

    pub(super) fn set_fingerprint_current(&self, project_path: &str, fingerprint: Option<String>) {
        if let Ok(mut guard) = self.states.lock() {
            if let Some(state) = guard.get_mut(project_path) {
                state.status.fingerprint_current = fingerprint;
                state.status.updated_at = chrono::Utc::now().timestamp_millis();
            }
        }
    }

    pub(super) fn report_progress(&self, project_path: &str, stage: &str, done: u32, total: u32) {
        if let Ok(mut guard) = self.states.lock() {
            if let Some(state) = guard.get_mut(project_path) {
                state.status.progress = Some(BuildProgress {
                    stage: stage.to_string(),
                    done,
                    total,
                });
                state.status.updated_at = chrono::Utc::now().timestamp_millis();
            }
        }
    }

    pub(super) fn mark_ready(&self, project_path: &str) {
        let indexed = read_indexed_fingerprint(project_path);
        if let Ok(mut guard) = self.states.lock() {
            if let Some(state) = guard.get_mut(project_path) {
                state.status.state = IndexState::Ready;
                state.status.progress = None;
                state.status.last_error = None;
                state.status.fingerprint_indexed = indexed;
                state.status.bm25_ready = state.status.fingerprint_indexed.is_some();
                state.status.vectors_ready = vecs_f32_path(&index_root(project_path)).exists();
                state.status.updated_at = chrono::Utc::now().timestamp_millis();
            }
        }
    }

    pub(super) fn mark_failed(&self, project_path: &str, error: String) {
        if let Ok(mut guard) = self.states.lock() {
            if let Some(state) = guard.get_mut(project_path) {
                state.status.state = IndexState::Failed;
                state.status.progress = None;
                state.status.last_error = Some(error);
                state.status.updated_at = chrono::Utc::now().timestamp_millis();
            }
        }
    }

    pub(super) fn mark_cancelled(&self, project_path: &str) {
        if let Ok(mut guard) = self.states.lock() {
            if let Some(state) = guard.get_mut(project_path) {
                state.status.state = IndexState::Failed;
                state.status.progress = None;
                state.status.last_error = Some("cancelled".to_string());
                state.status.updated_at = chrono::Utc::now().timestamp_millis();
            }
        }
    }

    pub(super) fn mark_ready_direct(
        &self,
        project: &str,
        current: Option<String>,
        indexed: Option<String>,
    ) {
        if let Ok(mut guard) = self.states.lock() {
            let state = guard
                .entry(project.to_string())
                .or_insert_with(ProjectIndexState::default);
            state.status.state = IndexState::Ready;
            state.status.fingerprint_current = current;
            state.status.fingerprint_indexed = indexed;
            state.status.bm25_ready = true;
            state.status.vectors_ready = vecs_f32_path(&index_root(project)).exists();
            state.status.progress = None;
            state.status.last_error = None;
            state.status.updated_at = chrono::Utc::now().timestamp_millis();
        }
    }

    pub(super) fn prepare_building(
        &self,
        project: &str,
        current: Option<String>,
        build_vectors: bool,
    ) -> Result<super::helpers::PrepareBuildResult, AppError> {
        let mut guard = self
            .states
            .lock()
            .map_err(|_| AppError::internal("E_TOOL_INTERNAL: index manager lock poisoned"))?;

        let state = guard
            .entry(project.to_string())
            .or_insert_with(ProjectIndexState::default);

        if matches!(state.status.state, IndexState::Building) {
            return Ok(super::helpers::PrepareBuildResult {
                already_building: true,
                cancel_flag: state.cancel.clone(),
            });
        }

        state.cancel.store(false, Ordering::Relaxed);
        state.status.state = IndexState::Building;
        state.status.last_error = None;
        state.status.fingerprint_current = current;
        state.status.vectors_ready = !build_vectors && vecs_f32_path(&index_root(project)).exists();
        state.status.progress = Some(super::helpers::make_initial_progress(build_vectors));
        state.status.updated_at = chrono::Utc::now().timestamp_millis();

        Ok(super::helpers::PrepareBuildResult {
            already_building: false,
            cancel_flag: state.cancel.clone(),
        })
    }

    pub(super) fn read_or_default_status(&self, project: &str) -> SearchIndexStatus {
        if let Ok(mut guard) = self.states.lock() {
            let entry = guard
                .entry(project.to_string())
                .or_insert_with(ProjectIndexState::default);
            return entry.status.clone();
        }

        SearchIndexStatus {
            state: IndexState::Failed,
            fingerprint_current: None,
            fingerprint_indexed: None,
            bm25_ready: false,
            vectors_ready: false,
            progress: None,
            last_error: Some("E_TOOL_INTERNAL: index manager lock poisoned".to_string()),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub(super) fn write_status(&self, project: &str, status: SearchIndexStatus) {
        if let Ok(mut guard) = self.states.lock() {
            if let Some(state) = guard.get_mut(project) {
                state.status = status;
            }
        }
    }
}

pub(super) fn read_indexed_fingerprint(project_path: &str) -> Option<String> {
    let root = index_root(project_path);
    let manifest_path = root.join("manifest.json");
    read_manifest(&manifest_path)
        .ok()
        .map(|manifest| manifest.corpus.fingerprint)
}

pub(super) struct IndexLockGuard {
    path: std::path::PathBuf,
}

impl Drop for IndexLockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub(super) fn acquire_index_lock(project_path: &str) -> Result<IndexLockGuard, AppError> {
    let lock_path = build_lock_path(project_path)?;
    create_lock_file(&lock_path)?;
    Ok(IndexLockGuard { path: lock_path })
}

fn build_lock_path(project_path: &str) -> Result<std::path::PathBuf, AppError> {
    let root = index_root(project_path);
    let lock_path = index_lock_path(&root);
    if let Some(parent) = lock_path.parent() {
        crate::services::ensure_dir(parent)?;
    }
    Ok(lock_path)
}

fn create_lock_file(lock_path: &std::path::PathBuf) -> Result<(), AppError> {
    let mut file = open_lock_file(lock_path)?;
    write_lock_payload(&mut file)
}

fn open_lock_file(lock_path: &std::path::PathBuf) -> Result<std::fs::File, AppError> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);

    options
        .open(lock_path)
        .map_err(|err| AppError::io_error(format!("failed to acquire search index lock: {err}")))
}

fn write_lock_payload(file: &mut std::fs::File) -> Result<(), AppError> {
    use std::io::Write;

    file.write_all(lock_payload().as_bytes())
        .map_err(|err| AppError::io_error(format!("failed to write index lock: {err}")))
}

fn lock_payload() -> String {
    let now = chrono::Utc::now().timestamp_millis();
    format!(
        "pid={:?}\nts={}\nbuild_id={}\n",
        std::process::id(),
        now,
        uuid::Uuid::new_v4()
    )
}
