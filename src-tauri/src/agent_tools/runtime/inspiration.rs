use std::time::Instant;

use serde_json::Value;

use crate::agent_tools::contracts::{FaultDomain, ToolResult};
use crate::application::command_usecases::inspiration::{
    apply_consensus_patch, apply_open_questions_patch, ApplyConsensusPatchInput,
    ApplyOpenQuestionsPatchInput,
};

use super::helpers::emit_from_result;
use super::input::classify_serde_error;

pub fn execute_consensus_patch(input: Value, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    let args: ApplyConsensusPatchInput = match serde_json::from_value(input) {
        Ok(value) => value,
        Err(error) => {
            let (code, message) = classify_serde_error(&error);
            let result = super::tool_err(
                "inspiration_consensus_patch",
                call_id,
                started,
                code,
                &message,
                false,
                FaultDomain::Validation,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let result = match apply_consensus_patch(args) {
        Ok(output) => super::tool_ok(
            "inspiration_consensus_patch",
            call_id,
            started,
            serde_json::to_value(output).expect("consensus patch output should serialize"),
            Some(vec!["inspiration:consensus".to_string()]),
            Some(vec!["inspiration:consensus".to_string()]),
        ),
        Err(error) => super::tool_err(
            "inspiration_consensus_patch",
            call_id,
            started,
            "E_INSPIRATION_CONSENSUS_PATCH_FAILED",
            &error.message,
            error.recoverable.unwrap_or(false),
            FaultDomain::Validation,
            Some(vec!["inspiration:consensus".to_string()]),
            Some(vec!["inspiration:consensus".to_string()]),
        ),
    };

    emit_from_result(&result, "execute");
    result
}

pub fn execute_open_questions_patch(input: Value, call_id: String) -> ToolResult<Value> {
    let started = Instant::now();
    let args: ApplyOpenQuestionsPatchInput = match serde_json::from_value(input) {
        Ok(value) => value,
        Err(error) => {
            let (code, message) = classify_serde_error(&error);
            let result = super::tool_err(
                "inspiration_open_questions_patch",
                call_id,
                started,
                code,
                &message,
                false,
                FaultDomain::Validation,
                None,
                None,
            );
            emit_from_result(&result, "execute");
            return result;
        }
    };

    let result = match apply_open_questions_patch(args) {
        Ok(output) => super::tool_ok(
            "inspiration_open_questions_patch",
            call_id,
            started,
            serde_json::to_value(output).expect("open questions patch output should serialize"),
            Some(vec!["inspiration:open_questions".to_string()]),
            Some(vec!["inspiration:open_questions".to_string()]),
        ),
        Err(error) => super::tool_err(
            "inspiration_open_questions_patch",
            call_id,
            started,
            "E_INSPIRATION_OPEN_QUESTIONS_PATCH_FAILED",
            &error.message,
            error.recoverable.unwrap_or(false),
            FaultDomain::Validation,
            Some(vec!["inspiration:open_questions".to_string()]),
            Some(vec!["inspiration:open_questions".to_string()]),
        ),
    };

    emit_from_result(&result, "execute");
    result
}
