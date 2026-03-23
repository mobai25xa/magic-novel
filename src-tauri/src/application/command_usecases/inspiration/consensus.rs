use crate::models::{AppError, ErrorCode};

use super::types::{
    ApplyConsensusPatchInput, ApplyConsensusPatchOutput, ConsensusPatchOperation, ConsensusValue,
};

pub fn apply_consensus_patch(
    input: ApplyConsensusPatchInput,
) -> Result<ApplyConsensusPatchOutput, AppError> {
    let mut state = input.state;
    let now = chrono::Utc::now().timestamp_millis();
    let field = state.field_mut(input.field_id);

    if field.locked {
        return Err(AppError {
            code: ErrorCode::Conflict,
            message: format!("consensus field '{}' is locked", input.field_id.as_str()),
            details: None,
            recoverable: Some(true),
        });
    }

    match input.operation {
        ConsensusPatchOperation::SetText => {
            if input.field_id.expects_list() {
                return Err(AppError::invalid_argument(format!(
                    "field '{}' requires list items",
                    input.field_id.as_str()
                )));
            }
            let value = ConsensusValue::from_text(input.text_value).ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "text_value is required for field '{}'",
                    input.field_id.as_str()
                ))
            })?;
            field.draft_value = Some(value);
        }
        ConsensusPatchOperation::SetItems => {
            if !input.field_id.expects_list() {
                return Err(AppError::invalid_argument(format!(
                    "field '{}' does not accept list items",
                    input.field_id.as_str()
                )));
            }
            let value = ConsensusValue::from_list(input.items).ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "items are required for field '{}'",
                    input.field_id.as_str()
                ))
            })?;
            field.draft_value = Some(value);
        }
        ConsensusPatchOperation::AppendItems => {
            if !input.field_id.expects_list() {
                return Err(AppError::invalid_argument(format!(
                    "field '{}' does not accept list append",
                    input.field_id.as_str()
                )));
            }

            let mut next_items = match field
                .draft_value
                .as_ref()
                .or(field.confirmed_value.as_ref())
            {
                Some(ConsensusValue::List(existing)) => existing.clone(),
                Some(ConsensusValue::Text(_)) => {
                    return Err(AppError::invalid_argument(format!(
                        "field '{}' contains invalid non-list state",
                        input.field_id.as_str()
                    )))
                }
                None => Vec::new(),
            };
            next_items.extend(input.items);
            let value = ConsensusValue::from_list(next_items).ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "items are required for field '{}'",
                    input.field_id.as_str()
                ))
            })?;
            field.draft_value = Some(value);
        }
        ConsensusPatchOperation::ClearDraft => {
            field.draft_value = None;
        }
    }

    field.updated_at = now;
    field.last_source_turn_id = input.source_turn_id;

    Ok(ApplyConsensusPatchOutput {
        field_id: input.field_id,
        operation: input.operation,
        updated_field: field.clone(),
        state,
    })
}
