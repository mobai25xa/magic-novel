use crate::models::{AppError, ErrorCode};

use super::types::{
    ApplyOpenQuestionsPatchInput, ApplyOpenQuestionsPatchOutput, OpenQuestion,
    OpenQuestionImportance, OpenQuestionStatus, OpenQuestionsPatchOperation,
};

pub fn apply_open_questions_patch(
    input: ApplyOpenQuestionsPatchInput,
) -> Result<ApplyOpenQuestionsPatchOutput, AppError> {
    let mut questions = input.questions;

    match input.operation {
        OpenQuestionsPatchOperation::Add => {
            let question = input
                .question
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::invalid_argument("question is required when adding"))?;
            let normalized = question.to_string();

            if let Some(existing) = questions.iter_mut().find(|candidate| {
                candidate.question.trim() == normalized
                    && candidate.status == OpenQuestionStatus::Open
            }) {
                existing.importance = input.importance.unwrap_or(existing.importance);
                return Ok(ApplyOpenQuestionsPatchOutput {
                    operation: input.operation,
                    updated_question: existing.clone(),
                    questions,
                });
            }

            let next = OpenQuestion {
                question_id: next_question_id(&questions, input.question_id.as_deref()),
                question: normalized,
                importance: input.importance.unwrap_or(OpenQuestionImportance::Medium),
                status: OpenQuestionStatus::Open,
            };
            questions.push(next.clone());
            Ok(ApplyOpenQuestionsPatchOutput {
                operation: input.operation,
                updated_question: next,
                questions,
            })
        }
        OpenQuestionsPatchOperation::Resolve | OpenQuestionsPatchOperation::Dismiss => {
            let question_id = input
                .question_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| AppError::invalid_argument("question_id is required"))?;
            let next_status = match input.operation {
                OpenQuestionsPatchOperation::Resolve => OpenQuestionStatus::Resolved,
                OpenQuestionsPatchOperation::Dismiss => OpenQuestionStatus::Dismissed,
                OpenQuestionsPatchOperation::Add => unreachable!("handled above"),
            };
            let target = questions
                .iter_mut()
                .find(|question| question.question_id == question_id)
                .ok_or_else(|| AppError {
                    code: ErrorCode::NotFound,
                    message: format!("open question not found: {question_id}"),
                    details: None,
                    recoverable: Some(true),
                })?;
            target.status = next_status;

            Ok(ApplyOpenQuestionsPatchOutput {
                operation: input.operation,
                updated_question: target.clone(),
                questions,
            })
        }
    }
}

fn next_question_id(existing: &[OpenQuestion], explicit: Option<&str>) -> String {
    if let Some(explicit) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return explicit.to_string();
    }

    let mut max_id = 0_u32;
    for question in existing {
        let Some(raw) = question.question_id.strip_prefix("q_") else {
            continue;
        };
        let Ok(value) = raw.parse::<u32>() else {
            continue;
        };
        max_id = max_id.max(value);
    }

    format!("q_{:03}", max_id + 1)
}
