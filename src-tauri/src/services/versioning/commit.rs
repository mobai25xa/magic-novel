use crate::models::AppError;
use crate::services::versioning_port::{EntityHead, VcCommitInput, VcCommitOutput, VcCommitPort};

use super::core::{
    append_call_index,
    append_wal,
    ensure_vc_layout,
    find_call_index_by_call_id,
    find_commit_after_hash_by_tx_id,
    load_or_init_head_state,
    maybe_snapshot,
    new_tx_id,
    vc_root_dir,
    write_head_state_atomic,
    write_revision_record,
    CallIndexRecord,
    HeadState,
    VersioningService,
    WalRecord,
    WalStatus,
    WalType,
    CALL_INDEX_FILE,
    HEAD_FILE,
    WAL_FILE,
};
use super::core::{
    current_head_from_entity_file,
    entity_path_from_entity_id,
    lock_path,
    write_entity_json_atomic,
    FileLockGuard,
};

pub fn commit_with_occ_impl(
    service: &VersioningService,
    input: VcCommitInput,
) -> Result<VcCommitOutput, AppError> {
    let _ = service;
    let setup = prepare_commit_context(&input)?;
    execute_commit_flow(&input, setup)
}

type CommitSetup = {
    vc_root: std::path::PathBuf,
    call_index_path: std::path::PathBuf,
    _lock_guard: FileLockGuard,
    head_state: HeadState,
    current: EntityHead,
    tx_id: String,
    next_event_seq: i64,
    now: i64,
}

fn prepare_commit_context(input: &VcCommitInput) -> Result<CommitSetup, AppError> {
    let vc_root = vc_root_dir(&input.project_path);
    ensure_vc_layout(&vc_root)?;

    let entity_lock_path = lock_path(&vc_root, &input.entity_id);
    let lock_guard = FileLockGuard::acquire(&entity_lock_path)?;

    let call_index_path = vc_root.join(CALL_INDEX_FILE);
    let mut head_state = load_or_init_head_state(&vc_root, &input.project_path)?;
    let current = current_head_from_entity_file(
        &input.project_path,
        &input.entity_id,
        head_state.entities.get(&input.entity_id),
    )?;

    ensure_occ_match(input, &current)?;

    let tx_id = new_tx_id();
    let next_event_seq = head_state.last_event_seq + 1;
    let now = chrono::Utc::now().timestamp_millis();

    Ok(CommitSetup {
        vc_root,
        call_index_path,
        _lock_guard: lock_guard,
        head_state,
        current,
        tx_id,
        next_event_seq,
        now,
    })
}

fn execute_commit_flow(input: &VcCommitInput, mut setup: CommitSetup) -> Result<VcCommitOutput, AppError> {
    let start = std::time::Instant::now();

    if let Some(existing) = find_call_index_by_call_id(&setup.call_index_path, &input.call_id)? {
        let after_hash = find_commit_after_hash_by_tx_id(&setup.vc_root.join(WAL_FILE), &existing.tx_id)?
            .unwrap_or_default();
        return Ok(VcCommitOutput {
            ok: true,
            tx_id: existing.tx_id,
            revision_before: existing.from_revision,
            revision_after: existing.to_revision,
            after_hash,
        });
    }

    append_begin_record(
        &setup.vc_root,
        input,
        &setup.tx_id,
        &setup.current,
        setup.next_event_seq,
        setup.now,
    )?;

    let after_hash = persist_commit_payload(&setup.vc_root, input, &setup.current)?;
    let commit_event_seq = setup.next_event_seq + 1;

    update_head_state(
        &setup.vc_root,
        input,
        &mut setup.head_state,
        &setup.tx_id,
        &after_hash,
        commit_event_seq,
        setup.now,
    )?;

    append_commit_record(
        &setup.vc_root,
        input,
        &setup.tx_id,
        &setup.current,
        &after_hash,
        commit_event_seq,
        setup.now,
        start.elapsed().as_millis(),
    )?;

    append_call_index(
        &setup.call_index_path,
        &CallIndexRecord {
            call_id: input.call_id.clone(),
            entity_id: input.entity_id.clone(),
            tx_id: setup.tx_id.clone(),
            from_revision: setup.current.revision,
            to_revision: setup.current.revision + 1,
            event_seq: commit_event_seq,
            ts: setup.now,
        },
    )?;

    maybe_snapshot(
        &setup.vc_root,
        &input.project_path,
        &input.entity_id,
        setup.current.revision + 1,
        setup.now,
    )?;

    Ok(VcCommitOutput {
        ok: true,
        tx_id: setup.tx_id,
        revision_before: setup.current.revision,
        revision_after: setup.current.revision + 1,
        after_hash,
    })
}

fn ensure_occ_match(input: &VcCommitInput, current: &EntityHead) -> Result<(), AppError> {
    if input.expected_revision != current.revision {
        return Err(super::core::app_err_vc(
            "E_VC_CONFLICT_REVISION",
            format!(
                "expected_revision {} does not match current_revision {}",
                input.expected_revision, current.revision
            ),
            false,
        ));
    }

    if !current.json_hash.is_empty() && input.before_hash != current.json_hash {
        return Err(super::core::app_err_vc(
            "E_VC_CONFLICT_REVISION",
            "before_hash does not match current head".to_string(),
            false,
        ));
    }

    Ok(())
}

fn append_begin_record(
    vc_root: &std::path::Path,
    input: &VcCommitInput,
    tx_id: &str,
    current: &EntityHead,
    event_seq: i64,
    now: i64,
) -> Result<(), AppError> {
    append_wal(
        &vc_root.join(WAL_FILE),
        &WalRecord {
            event_seq,
            ts: now,
            r#type: WalType::Begin,
            tx_id: tx_id.to_string(),
            call_id: Some(input.call_id.clone()),
            actor: Some(input.actor),
            entity_id: input.entity_id.clone(),
            from_revision: Some(current.revision),
            to_revision: Some(current.revision + 1),
            expected_revision: Some(input.expected_revision),
            before_hash: Some(input.before_hash.clone()),
            after_hash: None,
            patch_hash: None,
            status: WalStatus::Ok,
            detail: None,
        },
    )
}

fn persist_commit_payload(
    vc_root: &std::path::Path,
    input: &VcCommitInput,
    current: &EntityHead,
) -> Result<String, AppError> {
    let entity_path = entity_path_from_entity_id(&input.project_path, &input.entity_id)?;
    let after_hash = super::core::compute_json_hash(&input.after_json);

    write_entity_json_atomic(&entity_path, &input.after_json)?;
    write_revision_record(
        vc_root,
        &input.entity_id,
        current.revision + 1,
        &input.after_json,
        &after_hash,
    )?;

    Ok(after_hash)
}

fn update_head_state(
    vc_root: &std::path::Path,
    input: &VcCommitInput,
    head_state: &mut HeadState,
    tx_id: &str,
    after_hash: &str,
    commit_event_seq: i64,
    now: i64,
) -> Result<(), AppError> {
    let previous_snapshot_at = head_state
        .entities
        .get(&input.entity_id)
        .and_then(|head| head.last_snapshot_at);

    head_state.last_event_seq = commit_event_seq;
    head_state.entities.insert(
        input.entity_id.clone(),
        EntityHead {
            revision: input.expected_revision + 1,
            json_hash: after_hash.to_string(),
            last_call_id: Some(input.call_id.clone()),
            last_tx_id: Some(tx_id.to_string()),
            updated_at: now,
            last_snapshot_at: previous_snapshot_at,
        },
    );

    write_head_state_atomic(&vc_root.join(HEAD_FILE), head_state)
}

fn append_commit_record(
    vc_root: &std::path::Path,
    input: &VcCommitInput,
    tx_id: &str,
    current: &EntityHead,
    after_hash: &str,
    event_seq: i64,
    now: i64,
    duration_ms: u128,
) -> Result<(), AppError> {
    append_wal(
        &vc_root.join(WAL_FILE),
        &WalRecord {
            event_seq,
            ts: now,
            r#type: WalType::Commit,
            tx_id: tx_id.to_string(),
            call_id: Some(input.call_id.clone()),
            actor: Some(input.actor),
            entity_id: input.entity_id.clone(),
            from_revision: Some(current.revision),
            to_revision: Some(current.revision + 1),
            expected_revision: Some(input.expected_revision),
            before_hash: Some(input.before_hash.clone()),
            after_hash: Some(after_hash.to_string()),
            patch_hash: Some(super::core::compute_patch_hash(&input.patch_ops)),
            status: WalStatus::Ok,
            detail: Some(format!("duration_ms={duration_ms}")),
        },
    )
}

impl VcCommitPort for VersioningService {
    fn commit_with_occ(&self, input: VcCommitInput) -> Result<VcCommitOutput, AppError> {
        commit_with_occ_impl(self, input)
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
        input: crate::services::versioning_port::RollbackByRevisionInput,
    ) -> Result<crate::services::versioning_port::RollbackOutput, AppError> {
        super::core::rollback_by_revision_impl(self, input)
    }

    fn rollback_by_call_id(
        &self,
        input: crate::services::versioning_port::RollbackByCallIdInput,
    ) -> Result<crate::services::versioning_port::RollbackOutput, AppError> {
        super::core::rollback_by_call_id_impl(self, input)
    }

    fn recover(&self, project_path: &str) -> Result<crate::services::versioning_port::RecoverOutput, AppError> {
        super::recovery::recover_project(project_path)
    }
}
