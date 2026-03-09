use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::models::AppError;

use super::super::bm25::build::ensure_bm25_index;
use super::super::vector::{ensure_embedding_search_enabled, ensure_vector_index};
use super::corpus::{fingerprint_corpus, scan_corpus};
use super::manager::{BuildProgress, EnsureReason, EnsureResult, SearchIndexManager};
use super::paths::{index_root, vecs_f32_path};

pub(super) fn ensure_index_impl(
    manager: &SearchIndexManager,
    project: &str,
    reason: EnsureReason,
) -> Result<EnsureResult, AppError> {
    let manifest = super::manager::read_indexed_fingerprint(project);
    let current = scan_corpus(project)
        .map(|corpus| fingerprint_corpus(&corpus))
        .ok();

    let build_vectors = matches!(reason, EnsureReason::Warmup | EnsureReason::Rebuild);
    let force_rebuild = matches!(reason, EnsureReason::Rebuild);
    let vectors_ready = vecs_f32_path(&index_root(project)).exists();

    if !force_rebuild
        && manifest.is_some()
        && manifest == current
        && (!build_vectors || vectors_ready)
    {
        manager.mark_ready_direct(project, current, manifest);
        return Ok(EnsureResult::Ready);
    }

    let setup = manager.prepare_building(project, current, build_vectors)?;
    if setup.already_building {
        return Ok(EnsureResult::AlreadyBuilding);
    }

    spawn_build_thread(project.to_string(), setup.cancel_flag, build_vectors);
    Ok(EnsureResult::Started)
}

pub(super) fn snapshot_status(
    manager: &SearchIndexManager,
    project: &str,
) -> super::manager::SearchIndexStatus {
    let mut status = manager.read_or_default_status(project);
    status.fingerprint_indexed = super::manager::read_indexed_fingerprint(project);
    status.bm25_ready = status.fingerprint_indexed.is_some();
    status.vectors_ready = vecs_f32_path(&index_root(project)).exists();

    if matches!(status.state, super::manager::IndexState::Ready)
        && status.bm25_ready
        && !status.vectors_ready
    {
        status.state = super::manager::IndexState::Missing;
    }

    status.updated_at = chrono::Utc::now().timestamp_millis();
    manager.write_status(project, status.clone());
    status
}

#[derive(Clone)]
pub(super) struct PrepareBuildResult {
    pub already_building: bool,
    pub cancel_flag: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone)]
struct BuildPlan {
    build_vectors: bool,
}

pub(super) fn make_initial_progress(build_vectors: bool) -> BuildProgress {
    BuildProgress {
        stage: "scan".to_string(),
        done: 0,
        total: if build_vectors { 4 } else { 3 },
    }
}

fn spawn_build_thread(
    project_owned: String,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    build_vectors: bool,
) {
    std::thread::spawn(move || {
        let manager = SearchIndexManager::global();
        let plan = BuildPlan { build_vectors };

        let total_steps = if plan.build_vectors { 4 } else { 3 };
        manager.report_progress(&project_owned, "scan", 1, total_steps);

        if cancel_flag.load(Ordering::Relaxed) {
            manager.mark_cancelled(&project_owned);
            return;
        }

        let current_fingerprint = scan_corpus(&project_owned)
            .map(|corpus| fingerprint_corpus(&corpus))
            .ok();
        manager.set_fingerprint_current(&project_owned, current_fingerprint);
        manager.report_progress(&project_owned, "bm25", 2, total_steps);

        if cancel_flag.load(Ordering::Relaxed) {
            manager.mark_cancelled(&project_owned);
            return;
        }

        let lock = match super::manager::acquire_index_lock(&project_owned) {
            Ok(lock) => lock,
            Err(err) => {
                manager.mark_failed(&project_owned, err.message);
                return;
            }
        };

        let result = run_build_plan(&project_owned, &plan, &cancel_flag, total_steps, manager);
        drop(lock);

        match result {
            Ok(EnsureResult::Ready) => manager.mark_ready(&project_owned),
            Ok(EnsureResult::Cancelled) => manager.mark_cancelled(&project_owned),
            Ok(_) => manager.mark_ready(&project_owned),
            Err(err) => manager.mark_failed(&project_owned, err.message),
        }
    });
}

fn run_build_plan(
    project_path: &str,
    plan: &BuildPlan,
    cancel_flag: &Arc<std::sync::atomic::AtomicBool>,
    total_steps: u32,
    manager: &SearchIndexManager,
) -> Result<EnsureResult, AppError> {
    ensure_bm25_index(project_path)?;

    if cancel_flag.load(Ordering::Relaxed) {
        return Ok(EnsureResult::Cancelled);
    }

    if plan.build_vectors {
        manager.report_progress(project_path, "vectors", 3, total_steps);
        let settings = crate::services::load_openai_search_settings()?;

        if let Err(err) = ensure_embedding_search_enabled(&settings) {
            if err.recoverable.unwrap_or(false) {
                manager.report_progress(project_path, "commit", total_steps, total_steps);
                return Ok(EnsureResult::Ready);
            }
            return Err(err);
        }

        let _ = ensure_vector_index(project_path)?;

        if cancel_flag.load(Ordering::Relaxed) {
            return Ok(EnsureResult::Cancelled);
        }

        manager.report_progress(project_path, "commit", total_steps, total_steps);
        return Ok(EnsureResult::Ready);
    }

    manager.report_progress(project_path, "commit", 3, total_steps);
    Ok(EnsureResult::Ready)
}
