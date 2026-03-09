use crate::models::AppError;
use crate::services::versioning_port::{EntityHead, VcCommitInput, VcCommitOutput};

use super::core_utils::app_err_vc;

pub(crate) fn load_existing_call_output(err: &AppError) -> Option<VcCommitOutput> {
    if !err
        .message
        .starts_with("E_VC_DUP_CALL_ID: duplicate_call_id:")
    {
        return None;
    }

    let payload = err
        .message
        .strip_prefix("E_VC_DUP_CALL_ID: duplicate_call_id:")?
        .trim();
    let mut parts = payload.splitn(4, "::");

    Some(VcCommitOutput {
        ok: true,
        tx_id: parts.next()?.to_string(),
        revision_before: parts.next()?.parse::<i64>().ok()?,
        revision_after: parts.next()?.parse::<i64>().ok()?,
        after_hash: parts.next()?.to_string(),
    })
}

pub(crate) fn validate_commit_input(
    input: &VcCommitInput,
    current: &EntityHead,
) -> Result<(), AppError> {
    if input.expected_revision != current.revision {
        return Err(app_err_vc(
            "E_VC_CONFLICT_REVISION",
            format!(
                "expected_revision {} does not match current_revision {}",
                input.expected_revision, current.revision
            ),
            false,
        ));
    }

    if !current.json_hash.is_empty() && input.before_hash != current.json_hash {
        return Err(app_err_vc(
            "E_VC_CONFLICT_REVISION",
            "before_hash does not match current head".to_string(),
            false,
        ));
    }

    Ok(())
}
