use crate::models::AppError;
use crate::services::versioning_port::{
    EntityHead, RollbackByCallIdInput, RollbackByRevisionInput, RollbackOutput, VcCommitInput,
    VcCommitOutput, VcCommitPort,
};

use super::core_call_index::{find_call_index_by_call_id_entity, find_entity_id_by_call_id};
use super::core_head_wal::{
    append_wal, current_head_from_entity_file, load_or_init_head_state, next_event_seq,
};
use super::core_layout::{ensure_vc_layout, vc_root_dir, CALL_INDEX_FILE, WAL_FILE};
use super::core_snapshot::reconstruct_entity_at_revision;
use super::core_types::{WalRecord, WalStatus, WalType};
use super::core_utils::app_err_vc;

pub(crate) fn rollback_by_revision_impl(
    service: &dyn VcCommitPort,
    input: RollbackByRevisionInput,
) -> Result<RollbackOutput, AppError> {
    let vc_root = vc_root_dir(&input.project_path);
    ensure_vc_layout(&vc_root)?;

    let head_before = load_head_before(&input, &vc_root)?;
    validate_target_revision(input.target_revision, head_before.revision)?;

    let commit_out = commit_target_revision(service, &input, &vc_root, &head_before)?;
    append_rollback_wal(&input, &vc_root, &head_before, &commit_out)?;

    Ok(RollbackOutput {
        ok: true,
        tx_id: commit_out.tx_id,
        revision_before: head_before.revision,
        revision_after: commit_out.revision_after,
        after_hash: commit_out.after_hash,
        rolled_back_to_revision: input.target_revision,
    })
}

pub(crate) fn rollback_by_call_id_impl(
    service: &dyn VcCommitPort,
    input: RollbackByCallIdInput,
) -> Result<RollbackOutput, AppError> {
    let vc_root = vc_root_dir(&input.project_path);
    ensure_vc_layout(&vc_root)?;

    let entity_id =
        find_entity_id_by_call_id(&vc_root.join(CALL_INDEX_FILE), &input.target_call_id)?
            .ok_or_else(|| {
                app_err_vc(
                    "E_VC_DUP_CALL_ID",
                    "target call_id not found".to_string(),
                    false,
                )
            })?;

    let target_record = find_call_index_by_call_id_entity(
        &vc_root.join(CALL_INDEX_FILE),
        &input.target_call_id,
        &entity_id,
    )?
    .ok_or_else(|| {
        app_err_vc(
            "E_VC_DUP_CALL_ID",
            "target call_id not found".to_string(),
            false,
        )
    })?;

    rollback_by_revision_impl(
        service,
        RollbackByRevisionInput {
            project_path: input.project_path,
            entity_id,
            target_revision: target_record.from_revision,
            call_id: input.call_id,
            actor: input.actor,
            reason: input.reason,
        },
    )
}

fn load_head_before(
    input: &RollbackByRevisionInput,
    vc_root: &std::path::Path,
) -> Result<EntityHead, AppError> {
    let head_state = load_or_init_head_state(vc_root, &input.project_path)?;
    current_head_from_entity_file(
        &input.project_path,
        &input.entity_id,
        head_state.entities.get(&input.entity_id),
    )
}

fn validate_target_revision(target_revision: i64, current_revision: i64) -> Result<(), AppError> {
    if target_revision < 0 {
        return Err(app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            "target_revision must be >= 0".to_string(),
            false,
        ));
    }

    if target_revision > current_revision {
        return Err(app_err_vc(
            "E_VC_IO_WRITE_FAIL",
            "target_revision is greater than current head revision".to_string(),
            false,
        ));
    }

    Ok(())
}

fn commit_target_revision(
    service: &dyn VcCommitPort,
    input: &RollbackByRevisionInput,
    vc_root: &std::path::Path,
    head_before: &EntityHead,
) -> Result<VcCommitOutput, AppError> {
    let target_json = reconstruct_entity_at_revision(
        vc_root,
        &input.project_path,
        &input.entity_id,
        input.target_revision,
    )?;

    service.commit_with_occ(VcCommitInput {
        project_path: input.project_path.clone(),
        entity_id: input.entity_id.clone(),
        expected_revision: head_before.revision,
        call_id: input.call_id.clone(),
        actor: input.actor,
        before_hash: head_before.json_hash.clone(),
        after_json: target_json,
        patch_ops: vec![],
    })
}

fn append_rollback_wal(
    input: &RollbackByRevisionInput,
    vc_root: &std::path::Path,
    head_before: &EntityHead,
    commit_out: &VcCommitOutput,
) -> Result<(), AppError> {
    append_wal(
        &vc_root.join(WAL_FILE),
        &WalRecord {
            event_seq: next_event_seq(vc_root, &input.project_path)?,
            ts: chrono::Utc::now().timestamp_millis(),
            r#type: WalType::Rollback,
            tx_id: commit_out.tx_id.clone(),
            call_id: Some(input.call_id.clone()),
            actor: Some(input.actor),
            entity_id: input.entity_id.clone(),
            from_revision: Some(head_before.revision),
            to_revision: Some(commit_out.revision_after),
            expected_revision: Some(head_before.revision),
            before_hash: Some(head_before.json_hash.clone()),
            after_hash: Some(commit_out.after_hash.clone()),
            patch_hash: None,
            status: WalStatus::Ok,
            detail: input.reason.clone(),
        },
    )
}
