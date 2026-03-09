use std::path::PathBuf;

use crate::models::AppError;
use crate::services::versioning_port::{
    EntityHead, RecoverOutput, RollbackByCallIdInput, RollbackByRevisionInput, RollbackOutput,
    VcCommitInput, VcCommitOutput, VcCommitPort,
};

pub(crate) use super::core_call_index::{
    append_call_index, find_call_index_by_call_id, find_commit_after_hash_by_tx_id,
};
pub(crate) use super::core_commit_support::{load_existing_call_output, validate_commit_input};
pub(crate) use super::core_head_wal::{
    append_wal, current_head_from_entity_file, load_or_init_head_state, next_event_seq,
    truncate_wal_if_corrupted, write_head_state_atomic,
};
pub(crate) use super::core_layout::{
    ensure_vc_layout, entity_path_from_entity_id, lock_path, vc_root_dir, CALL_INDEX_FILE,
    HEAD_FILE, WAL_FILE,
};
pub(crate) use super::core_rebuild::{
    cleanup_tmp_files, rebuild_call_index_from_wal, rebuild_head_from_wal,
};
pub(crate) use super::core_revision::{write_entity_json_atomic, write_revision_record};
pub(crate) use super::core_rollback::{rollback_by_call_id_impl, rollback_by_revision_impl};
pub(crate) use super::core_snapshot::maybe_snapshot;
pub(crate) use super::core_types::{CallIndexRecord, HeadState, WalRecord, WalStatus, WalType};
pub(crate) use super::core_utils::{
    app_err_vc, compute_json_hash, compute_patch_hash, new_tx_id, FileLockGuard,
};

pub struct VersioningService;

impl VersioningService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VersioningService {
    fn default() -> Self {
        Self::new()
    }
}

impl VcCommitPort for VersioningService {
    fn commit_with_occ(&self, input: VcCommitInput) -> Result<VcCommitOutput, AppError> {
        let start = std::time::Instant::now();
        let mut context = match build_commit_context(&input) {
            Ok(context) => context,
            Err(err) => {
                if let Some(existing) = load_existing_call_output(&err) {
                    return Ok(existing);
                }
                return Err(err);
            }
        };

        commit_entity_state(&input, &mut context, start)
    }

    fn get_current_head(
        &self,
        project_path: &str,
        entity_id: &str,
    ) -> Result<EntityHead, AppError> {
        let vc_root = vc_root_dir(project_path);
        ensure_vc_layout(&vc_root)?;
        let head_state = load_or_init_head_state(&vc_root, project_path)?;
        current_head_from_entity_file(project_path, entity_id, head_state.entities.get(entity_id))
    }

    fn rollback_by_revision(
        &self,
        input: RollbackByRevisionInput,
    ) -> Result<RollbackOutput, AppError> {
        rollback_by_revision_impl(self, input)
    }

    fn rollback_by_call_id(
        &self,
        input: RollbackByCallIdInput,
    ) -> Result<RollbackOutput, AppError> {
        rollback_by_call_id_impl(self, input)
    }

    fn recover(&self, project_path: &str) -> Result<RecoverOutput, AppError> {
        super::recovery::recover_project(project_path)
    }
}

struct CommitContext {
    vc_root: PathBuf,
    call_index_path: PathBuf,
    _lock_guard: FileLockGuard,
    head_state: HeadState,
    current: EntityHead,
    tx_id: String,
    next_event_seq: i64,
    now: i64,
}

fn build_commit_context(input: &VcCommitInput) -> Result<CommitContext, AppError> {
    let vc_root = vc_root_dir(&input.project_path);
    ensure_vc_layout(&vc_root)?;

    let entity_lock_path = lock_path(&vc_root, &input.entity_id);
    let lock_guard = FileLockGuard::acquire(&entity_lock_path)?;

    let call_index_path = vc_root.join(CALL_INDEX_FILE);
    if let Some(existing) = find_call_index_by_call_id(&call_index_path, &input.call_id)? {
        let after_hash = find_commit_after_hash_by_tx_id(&vc_root.join(WAL_FILE), &existing.tx_id)?
            .unwrap_or_default();
        return Err(app_err_vc(
            "E_VC_DUP_CALL_ID",
            format!(
                "duplicate_call_id:{}::{}::{}::{}",
                existing.tx_id, existing.from_revision, existing.to_revision, after_hash
            ),
            true,
        ));
    }

    let head_state = load_or_init_head_state(&vc_root, &input.project_path)?;
    let current = current_head_from_entity_file(
        &input.project_path,
        &input.entity_id,
        head_state.entities.get(&input.entity_id),
    )?;

    validate_commit_input(input, &current)?;

    let next_event_seq = head_state.last_event_seq + 1;

    Ok(CommitContext {
        vc_root,
        call_index_path,
        _lock_guard: lock_guard,
        head_state,
        current,
        tx_id: new_tx_id(),
        next_event_seq,
        now: chrono::Utc::now().timestamp_millis(),
    })
}

fn commit_entity_state(
    input: &VcCommitInput,
    context: &mut CommitContext,
    start: std::time::Instant,
) -> Result<VcCommitOutput, AppError> {
    append_begin_record(input, context)?;
    let after_hash = persist_entity_and_revision(input, context)?;
    update_head_after_commit(input, context, &after_hash)?;
    append_commit_record(input, context, &after_hash, start.elapsed().as_millis())?;

    append_call_index(
        &context.call_index_path,
        &CallIndexRecord {
            call_id: input.call_id.clone(),
            entity_id: input.entity_id.clone(),
            tx_id: context.tx_id.clone(),
            from_revision: context.current.revision,
            to_revision: context.current.revision + 1,
            event_seq: context.next_event_seq + 1,
            ts: context.now,
        },
    )?;

    maybe_snapshot(
        &context.vc_root,
        &input.project_path,
        &input.entity_id,
        context.current.revision + 1,
        context.now,
    )?;

    Ok(VcCommitOutput {
        ok: true,
        tx_id: context.tx_id.clone(),
        revision_before: context.current.revision,
        revision_after: context.current.revision + 1,
        after_hash,
    })
}

fn append_begin_record(input: &VcCommitInput, context: &CommitContext) -> Result<(), AppError> {
    append_wal(
        &context.vc_root.join(WAL_FILE),
        &WalRecord {
            event_seq: context.next_event_seq,
            ts: context.now,
            r#type: WalType::Begin,
            tx_id: context.tx_id.clone(),
            call_id: Some(input.call_id.clone()),
            actor: Some(input.actor),
            entity_id: input.entity_id.clone(),
            from_revision: Some(context.current.revision),
            to_revision: Some(context.current.revision + 1),
            expected_revision: Some(input.expected_revision),
            before_hash: Some(input.before_hash.clone()),
            after_hash: None,
            patch_hash: None,
            status: WalStatus::Ok,
            detail: None,
        },
    )
}

fn persist_entity_and_revision(
    input: &VcCommitInput,
    context: &CommitContext,
) -> Result<String, AppError> {
    let entity_path = entity_path_from_entity_id(&input.project_path, &input.entity_id)?;
    let after_hash = compute_json_hash(&input.after_json);

    write_entity_json_atomic(&entity_path, &input.after_json)?;
    write_revision_record(
        &context.vc_root,
        &input.entity_id,
        context.current.revision + 1,
        &input.after_json,
        &after_hash,
    )?;

    Ok(after_hash)
}

fn update_head_after_commit(
    input: &VcCommitInput,
    context: &mut CommitContext,
    after_hash: &str,
) -> Result<(), AppError> {
    context.head_state.last_event_seq = context.next_event_seq + 1;
    context.head_state.entities.insert(
        input.entity_id.clone(),
        EntityHead {
            revision: context.current.revision + 1,
            json_hash: after_hash.to_string(),
            last_call_id: Some(input.call_id.clone()),
            last_tx_id: Some(context.tx_id.clone()),
            updated_at: context.now,
            last_snapshot_at: context.current.last_snapshot_at,
        },
    );

    write_head_state_atomic(&context.vc_root.join(HEAD_FILE), &context.head_state)
}

fn append_commit_record(
    input: &VcCommitInput,
    context: &CommitContext,
    after_hash: &str,
    duration_ms: u128,
) -> Result<(), AppError> {
    append_wal(
        &context.vc_root.join(WAL_FILE),
        &WalRecord {
            event_seq: context.next_event_seq + 1,
            ts: context.now,
            r#type: WalType::Commit,
            tx_id: context.tx_id.clone(),
            call_id: Some(input.call_id.clone()),
            actor: Some(input.actor),
            entity_id: input.entity_id.clone(),
            from_revision: Some(context.current.revision),
            to_revision: Some(context.current.revision + 1),
            expected_revision: Some(input.expected_revision),
            before_hash: Some(input.before_hash.clone()),
            after_hash: Some(after_hash.to_string()),
            patch_hash: Some(compute_patch_hash(&input.patch_ops)),
            status: WalStatus::Ok,
            detail: Some(format!("duration_ms={duration_ms}")),
        },
    )
}
