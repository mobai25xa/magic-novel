//! Agent Engine - Core loop (round-driven agent execution)
//!
//! Aligned with docs/magic_plan/plan_agent/12-agent-loop-and-turn-engine-pseudocode.md

use std::time::Instant;

use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::agent_engine::session_state::{self, SuspendedTurnState};
use crate::commands::agent_engine::AgentEditorState;
use crate::models::AppError;
use crate::services::{
    load_openai_search_settings, save_runtime_snapshot_from_input, RuntimeSnapshotUpsertInput,
};

use super::compaction::{self, CompactionConfig, CompactionSummarizer, TruncationSummarizer};
use super::context_loader::{inject_unified_context, ContextCache};
use super::emitter::EventSink;
use super::llm_compaction_summarizer::LlmCompactionSummarizer;
use super::messages::{AgentMessage, ConversationState};
use super::tool_routing::resolve_turn_tool_exposure;
use super::tool_scheduler::ToolScheduler;
use super::turn::TurnEngine;
use super::types::{LoopConfig, StopReason, ToolCallInfo, DEFAULT_MODEL, DEFAULT_PROVIDER};
use super::worker_dispatch::{extract_worker_todo_items, has_worker_items};

/// Result from a complete agent loop run
#[derive(Debug, Clone)]
pub struct LoopResult {
    pub stop_reason: StopReason,
    pub rounds_executed: u32,
    pub total_tool_calls: u32,
    pub latency_ms: u64,
    pub active_skill: Option<String>,
}

/// The core agent loop: repeatedly calls LLM, executes tools, until done or suspended.
pub struct AgentLoop<S: EventSink> {
    emitter: S,
    config: LoopConfig,
    project_path: String,
    mission_id: Option<String>,
    cancel_token: CancellationToken,
    active_chapter_path: Option<String>,
    active_skill: Option<String>,
    worker_tool_whitelist: Option<Vec<String>>,
    /// Editor state snapshot from frontend
    editor_state: Option<AgentEditorState>,
    /// Provider info saved for suspend/resume (set by caller)
    pub provider_name: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

impl<S: EventSink> AgentLoop<S> {
    pub fn new(
        emitter: S,
        config: LoopConfig,
        project_path: String,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            emitter,
            config,
            project_path,
            mission_id: None,
            cancel_token,
            active_chapter_path: None,
            active_skill: None,
            worker_tool_whitelist: None,
            editor_state: None,
            provider_name: String::new(),
            model: String::new(),
            base_url: String::new(),
            api_key: String::new(),
        }
    }

    pub fn with_mission_id(mut self, mission_id: Option<String>) -> Self {
        self.mission_id = mission_id
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        self
    }

    /// Set provider info for suspend/resume state capture.
    pub fn with_provider_info(
        mut self,
        provider_name: String,
        model: String,
        base_url: String,
        api_key: String,
    ) -> Self {
        self.provider_name = provider_name;
        self.model = model;
        self.base_url = base_url;
        self.api_key = api_key;
        self
    }

    pub fn with_active_chapter_path(mut self, active_chapter_path: Option<String>) -> Self {
        self.active_chapter_path = active_chapter_path
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        self
    }

    pub fn with_active_skill(mut self, skill: Option<String>) -> Self {
        self.active_skill = skill
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        self
    }

    pub fn with_editor_state(mut self, editor_state: Option<AgentEditorState>) -> Self {
        self.editor_state = editor_state;
        self
    }

    pub fn with_tool_whitelist(mut self, tool_whitelist: Option<Vec<String>>) -> Self {
        self.worker_tool_whitelist = tool_whitelist.and_then(|tools| {
            let filtered: Vec<String> = tools
                .into_iter()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            if filtered.is_empty() {
                None
            } else {
                Some(filtered)
            }
        });
        self
    }

    /// Run the agent loop until completion, error, or suspension.
    pub async fn run(
        &self,
        state: &mut ConversationState,
        turn_engine: &dyn TurnEngine,
    ) -> Result<LoopResult, AppError> {
        let loop_start = Instant::now();
        let mut rounds_executed = 0_u32;
        let mut total_tool_calls = 0_u32;
        let mut active_skill: Option<String> = self.active_skill.clone();
        let mut plan_policy_corrected_this_turn = false;
        let semantic_retrieval_enabled = load_openai_search_settings()
            .map(|settings| settings.openai_embedding_enabled)
            .unwrap_or(false);
        let resolved_tool_exposure = resolve_turn_tool_exposure(
            state,
            &self.config,
            self.active_chapter_path.as_deref(),
            self.worker_tool_whitelist.as_deref(),
            semantic_retrieval_enabled,
        );
        let tool_schema_bundle = &resolved_tool_exposure.bundle;
        let tool_exposure_payload = resolved_tool_exposure.telemetry.to_payload();
        let provider_name = if self.provider_name.trim().is_empty() {
            DEFAULT_PROVIDER.to_string()
        } else {
            self.provider_name.clone()
        };
        let model_name = if self.model.trim().is_empty() {
            DEFAULT_MODEL.to_string()
        } else {
            self.model.clone()
        };

        emit_tool_exposure_observability_metric(
            &state.session_id,
            &provider_name,
            &model_name,
            &resolved_tool_exposure.telemetry,
        );

        let _ = self.emitter.emit_raw(
            super::events::event_types::PLAN_STARTED,
            tool_exposure_payload.clone(),
        );
        let compaction_config = CompactionConfig {
            summary_provider: provider_name.clone(),
            summary_model: model_name.clone(),
            ..CompactionConfig::default()
        };
        let truncation_fallback = TruncationSummarizer;
        let llm_compaction = LlmCompactionSummarizer {
            provider: compaction_config.summary_provider.clone(),
            model: compaction_config.summary_model.clone(),
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
        };
        let using_compaction_fallback =
            self.base_url.trim().is_empty() || self.api_key.trim().is_empty();
        let compaction_summarizer: &dyn CompactionSummarizer = if using_compaction_fallback {
            tracing::warn!(
                target: "agent_engine",
                "LLM compaction unavailable (missing base_url/api_key), using truncation fallback"
            );
            let _ = self.emitter.emit_raw(
                super::events::event_types::COMPACTION_FALLBACK,
                json!({
                    "reason": "missing_credentials",
                    "message": "Context compaction is using truncation fallback (without LLM summary), and some conversation details may be lost.",
                }),
            );
            &truncation_fallback
        } else {
            &llm_compaction
        };

        // Stale session detection
        const STALE_SESSION_THRESHOLD_MS: i64 = 30 * 60 * 1000;
        if let Some(last_msg) = state.messages.last() {
            let now_ms = chrono::Utc::now().timestamp_millis();
            let age_ms = now_ms - last_msg.ts;
            if age_ms > STALE_SESSION_THRESHOLD_MS
                && state.messages.len() > compaction_config.keep_recent_count
            {
                tracing::info!(
                    target: "agent_engine",
                    session_id = %state.session_id,
                    age_ms = age_ms,
                    "stale session detected, triggering compaction"
                );
                compaction::compact(
                    state,
                    &self.emitter,
                    compaction_summarizer,
                    &compaction_config,
                    "stale_session",
                )
                .await?;
            }
        }

        let mut context_cache = ContextCache::new();

        for _round in 0..self.config.max_rounds {
            rounds_executed += 1;

            if compaction::should_compact(state, &compaction_config) {
                compaction::compact(
                    state,
                    &self.emitter,
                    compaction_summarizer,
                    &compaction_config,
                    "threshold",
                )
                .await?;
            }

            // Inject unified [Context] system message before LLM call
            inject_unified_context(
                state,
                &self.project_path,
                &self.mission_id,
                &self.active_chapter_path,
                &active_skill,
                &self.editor_state,
                state.current_turn,
                &mut context_cache,
            );

            // Check cancellation before each round
            if self.cancel_token.is_cancelled() {
                let latency_ms = loop_start.elapsed().as_millis() as u64;
                let mut payload = with_turn_outcome_meta(
                    &tool_exposure_payload,
                    total_tool_calls,
                    rounds_executed,
                );
                if let Some(map) = payload.as_object_mut() {
                    map.insert("stop_reason".to_string(), json!(StopReason::Cancel));
                    map.insert("latency_ms".to_string(), json!(latency_ms));
                }
                self.emitter
                    .emit_raw(super::events::event_types::TURN_CANCELLED, payload)?;
                return Ok(LoopResult {
                    stop_reason: StopReason::Cancel,
                    rounds_executed,
                    total_tool_calls,
                    latency_ms,
                    active_skill: active_skill.clone(),
                });
            }

            // (A) Call LLM
            let turn_out = match turn_engine
                .execute_turn(state, &tool_schema_bundle.schemas)
                .await
            {
                Ok(out) => out,
                Err(e) => {
                    if compaction::is_context_limit_error(&e)
                        && compaction::compact(
                            state,
                            &self.emitter,
                            compaction_summarizer,
                            &compaction_config,
                            "context_limit",
                        )
                        .await
                        .is_ok()
                    {
                        continue;
                    }

                    let error_code = e
                        .details
                        .as_ref()
                        .and_then(|d| d.get("code"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("E_LLM_UNKNOWN");

                    if error_code == "E_CANCELLED" {
                        let latency_ms = loop_start.elapsed().as_millis() as u64;
                        let mut payload = with_turn_outcome_meta(
                            &tool_exposure_payload,
                            total_tool_calls,
                            rounds_executed,
                        );
                        if let Some(map) = payload.as_object_mut() {
                            map.insert("stop_reason".to_string(), json!(StopReason::Cancel));
                            map.insert("latency_ms".to_string(), json!(latency_ms));
                            map.insert("error_code".to_string(), json!("E_CANCELLED"));
                        }
                        self.emitter
                            .emit_raw(super::events::event_types::TURN_CANCELLED, payload)?;
                        return Ok(LoopResult {
                            stop_reason: StopReason::Cancel,
                            rounds_executed,
                            total_tool_calls,
                            latency_ms,
                            active_skill: active_skill.clone(),
                        });
                    }

                    let error_detail = enrich_turn_failure_detail(
                        e.details.clone(),
                        error_code,
                        &provider_name,
                        &model_name,
                        &resolved_tool_exposure.telemetry,
                        &tool_schema_bundle.exposed_tools,
                        &tool_schema_bundle.skipped_tools,
                        total_tool_calls,
                        rounds_executed,
                    );
                    emit_turn_failed_observability_metrics(
                        &state.session_id,
                        &provider_name,
                        &model_name,
                        &resolved_tool_exposure.telemetry,
                        error_code,
                        error_detail.as_ref(),
                        total_tool_calls,
                        rounds_executed,
                    );
                    self.emitter
                        .turn_failed(error_code, &e.message, error_detail)?;
                    return Err(e);
                }
            };

            // Persist assistant message to conversation state
            state.messages.push(turn_out.assistant_message.clone());
            if let Err(e) = self
                .emitter
                .persist_assistant_message(&turn_out.assistant_message, state.current_turn)
            {
                tracing::warn!(
                    target: "agent_engine",
                    error = %e,
                    "failed to persist assistant message"
                );
            }

            // Update usage
            if let Some(usage) = &turn_out.usage {
                self.emitter.usage_update(usage)?;
                state.last_usage = Some(usage.clone());
            }

            // (B) No tool calls -> done
            if turn_out.tool_calls.is_empty() {
                let latency_ms = loop_start.elapsed().as_millis() as u64;
                self.emitter.turn_completed_with_meta(
                    &turn_out.stop_reason,
                    latency_ms,
                    false,
                    Some(with_turn_outcome_meta(
                        &tool_exposure_payload,
                        total_tool_calls,
                        rounds_executed,
                    )),
                )?;
                return Ok(LoopResult {
                    stop_reason: turn_out.stop_reason,
                    rounds_executed,
                    total_tool_calls,
                    latency_ms,
                    active_skill: active_skill.clone(),
                });
            }

            // (C) Policy gate: Plan-before-Write (soft)
            if let Some(violation) = check_plan_before_write_policy(&turn_out.tool_calls) {
                let policy_msg = "Policy hint: for multi-step changes, call todowrite first with milestone-level, user-verifiable tasks.";

                tracing::warn!(
                    target: "agent_engine",
                    violating_tool = %violation.violating_tool,
                    violating_index = violation.violating_index,
                    "write tool called without prior todowrite in same round (soft policy)"
                );

                if !plan_policy_corrected_this_turn {
                    state
                        .messages
                        .push(AgentMessage::system(policy_msg.to_string()));
                    plan_policy_corrected_this_turn = true;
                }
            }

            // (D) Execute tools via scheduler
            let tool_calls_snapshot: Vec<ToolCallInfo> =
                turn_out.tool_calls.iter().cloned().collect();
            let scheduler = ToolScheduler::new(
                self.emitter.clone(),
                self.project_path.clone(),
                self.config.approval_mode,
                self.config.clarification_mode,
                self.cancel_token.clone(),
            )
            .with_active_chapter_path(self.active_chapter_path.clone())
            .with_active_skill(active_skill.clone())
            .with_allowed_tools(Some(tool_schema_bundle.exposed_tools.clone()));

            let exec_result = match scheduler.execute_batch(turn_out.tool_calls).await {
                Ok(res) => res,
                Err(e) => {
                    let error_code = e
                        .details
                        .as_ref()
                        .and_then(|d| d.get("code"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("E_TOOL_SCHEDULER_FAILED");

                    if error_code == "E_CANCELLED" {
                        let latency_ms = loop_start.elapsed().as_millis() as u64;
                        let mut payload = with_turn_outcome_meta(
                            &tool_exposure_payload,
                            total_tool_calls,
                            rounds_executed,
                        );
                        if let Some(map) = payload.as_object_mut() {
                            map.insert("stop_reason".to_string(), json!(StopReason::Cancel));
                            map.insert("latency_ms".to_string(), json!(latency_ms));
                            map.insert("error_code".to_string(), json!("E_CANCELLED"));
                        }
                        self.emitter
                            .emit_raw(super::events::event_types::TURN_CANCELLED, payload)?;
                        return Ok(LoopResult {
                            stop_reason: StopReason::Cancel,
                            rounds_executed,
                            total_tool_calls,
                            latency_ms,
                            active_skill: active_skill.clone(),
                        });
                    }

                    let error_detail = enrich_turn_failure_detail(
                        e.details.clone(),
                        error_code,
                        &provider_name,
                        &model_name,
                        &resolved_tool_exposure.telemetry,
                        &tool_schema_bundle.exposed_tools,
                        &tool_schema_bundle.skipped_tools,
                        total_tool_calls,
                        rounds_executed,
                    );
                    emit_turn_failed_observability_metrics(
                        &state.session_id,
                        &provider_name,
                        &model_name,
                        &resolved_tool_exposure.telemetry,
                        error_code,
                        error_detail.as_ref(),
                        total_tool_calls,
                        rounds_executed,
                    );
                    self.emitter
                        .turn_failed(error_code, &e.message, error_detail)?;
                    return Err(e);
                }
            };

            total_tool_calls += exec_result.executed_count;
            state.total_tool_calls = total_tool_calls;

            let tool_messages = exec_result.tool_messages;
            update_active_skill_from_tool_results(&tool_messages, &mut active_skill);

            // Invalidate context cache if create/edit tools were called
            for tc in &tool_calls_snapshot {
                if tc.tool_name == "create" {
                    context_cache.invalidate_project();
                }
                if tc.tool_name == "edit" {
                    if let Some(path) = tc.args.get("path").and_then(|v| v.as_str()) {
                        if path.starts_with(".magic_novel") {
                            context_cache.invalidate_rules();
                        }
                    }
                }
            }

            for msg in tool_messages {
                state.messages.push(msg);
            }

            // (D') Legacy todo-worker dispatch is retired. Keep parsing for compatibility,
            // but do not execute worker-assigned todo items.
            let todo_items = extract_worker_todo_items(&tool_calls_snapshot);
            if has_worker_items(&todo_items) {
                tracing::info!(
                    target: "agent_engine",
                    session_id = %state.session_id,
                    worker_items = todo_items.iter().filter(|i| i.worker.is_some()).count(),
                    "worker todo items detected but legacy dispatch is retired; ignoring"
                );
            }

            // Check safety valve
            if total_tool_calls >= self.config.max_tool_calls {
                tracing::warn!(
                    target: "agent_engine",
                    total_tool_calls,
                    max = self.config.max_tool_calls,
                    "max_tool_calls safety valve triggered"
                );
                let latency_ms = loop_start.elapsed().as_millis() as u64;
                self.emitter.turn_completed_with_meta(
                    &StopReason::Limit,
                    latency_ms,
                    false,
                    Some(with_turn_outcome_meta(
                        &tool_exposure_payload,
                        total_tool_calls,
                        rounds_executed,
                    )),
                )?;
                return Ok(LoopResult {
                    stop_reason: StopReason::Limit,
                    rounds_executed,
                    total_tool_calls,
                    latency_ms,
                    active_skill: active_skill.clone(),
                });
            }

            // (E) Check for suspension (confirmation/askuser)
            if let Some(ref suspend_info) = exec_result.suspend_reason {
                let session_id = &state.session_id;
                let suspended = SuspendedTurnState {
                    conversation_state: state.clone(),
                    pending_tool_call: suspend_info.pending_tool_call.clone(),
                    pending_call_id: suspend_info.pending_call_id.clone(),
                    remaining_tool_calls: suspend_info.remaining_tool_calls.clone(),
                    completed_messages: suspend_info.completed_messages.clone(),
                    loop_config: self.config.clone(),
                    project_path: self.project_path.clone(),
                    provider_name: self.provider_name.clone(),
                    model: self.model.clone(),
                    base_url: self.base_url.clone(),
                    api_key: self.api_key.clone(),
                    active_chapter_path: self.active_chapter_path.clone(),
                    active_skill: active_skill.clone(),
                    system_prompt: None,
                    suspend_reason: suspend_info.reason.clone(),
                    rounds_executed,
                    total_tool_calls,
                };

                let snapshot_input = RuntimeSnapshotUpsertInput::from_suspended(
                    session_id.to_string(),
                    suspended.clone(),
                    Some(state.current_turn),
                );
                if let Err(err) = save_runtime_snapshot_from_input(
                    std::path::Path::new(&self.project_path),
                    snapshot_input,
                ) {
                    tracing::warn!(
                        target: "agent_engine",
                        session_id = %session_id,
                        error = %err,
                        "failed to persist suspended runtime snapshot"
                    );
                }

                session_state::global().suspend_turn(session_id, suspended);

                let latency_ms = loop_start.elapsed().as_millis() as u64;
                self.emitter.turn_completed_with_meta(
                    &suspend_info.reason,
                    latency_ms,
                    false,
                    Some(with_turn_outcome_meta(
                        &tool_exposure_payload,
                        total_tool_calls,
                        rounds_executed,
                    )),
                )?;
                return Ok(LoopResult {
                    stop_reason: suspend_info.reason.clone(),
                    rounds_executed,
                    total_tool_calls,
                    latency_ms,
                    active_skill: active_skill.clone(),
                });
            }

            // (F) Continue to next round
        }

        // Max rounds exceeded
        let latency_ms = loop_start.elapsed().as_millis() as u64;
        self.emitter.turn_completed_with_meta(
            &StopReason::Limit,
            latency_ms,
            false,
            Some(with_turn_outcome_meta(
                &tool_exposure_payload,
                total_tool_calls,
                rounds_executed,
            )),
        )?;
        Ok(LoopResult {
            stop_reason: StopReason::Limit,
            rounds_executed,
            total_tool_calls,
            latency_ms,
            active_skill,
        })
    }
}

// ── Policy enforcement ──

#[derive(Debug, Clone)]
struct PlanBeforeWriteViolation {
    violating_tool: String,
    violating_index: usize,
}

fn check_plan_before_write_policy(tool_calls: &[ToolCallInfo]) -> Option<PlanBeforeWriteViolation> {
    let mut todowrite_seen = false;

    for (index, tc) in tool_calls.iter().enumerate() {
        if tc.tool_name == "todowrite" {
            if todowrite_call_is_valid(&tc.args) {
                todowrite_seen = true;
            }
            continue;
        }

        if is_write_tool(&tc.tool_name) && !todowrite_seen {
            return Some(PlanBeforeWriteViolation {
                violating_tool: tc.tool_name.clone(),
                violating_index: index,
            });
        }
    }

    None
}

fn todowrite_call_is_valid(args: &serde_json::Value) -> bool {
    super::todowrite::parse_todo_input(args, "policy_validation").is_ok()
}

fn is_write_tool(tool_name: &str) -> bool {
    matches!(tool_name, "edit" | "create")
}

fn with_turn_outcome_meta(
    tool_exposure_payload: &serde_json::Value,
    total_tool_calls: u32,
    rounds_executed: u32,
) -> serde_json::Value {
    let mut meta = tool_exposure_payload.clone();
    let Some(map) = meta.as_object_mut() else {
        return json!({
            "tool_exposure": tool_exposure_payload,
            "tool_call_count": total_tool_calls,
            "rounds_executed": rounds_executed,
        });
    };

    let fallback_occurred = map
        .get("fallback_from")
        .and_then(|value| value.as_str())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    map.insert("tool_call_count".to_string(), json!(total_tool_calls));
    map.insert("rounds_executed".to_string(), json!(rounds_executed));
    map.insert("fallback_occurred".to_string(), json!(fallback_occurred));

    meta
}

fn emit_tool_exposure_observability_metric(
    session_id: &str,
    provider_name: &str,
    model_name: &str,
    telemetry: &super::tool_routing::ToolExposureTelemetry,
) {
    tracing::info!(
        target: "agent_engine_observability",
        session_id = %session_id,
        provider = %provider_name,
        model = %model_name,
        tool_package = ?telemetry.tool_package,
        route_reason = %telemetry.route_reason,
        rollout_mode = %telemetry.rollout_mode,
        rollout_in_canary = telemetry.rollout_in_canary,
        canary_percent = ?telemetry.canary_percent,
        metric = "package_fallback_rate",
        value = if telemetry.fallback_from.is_some() { 1_u8 } else { 0_u8 },
        "tool exposure selected"
    );
}

fn detail_string(
    detail_map: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<String> {
    detail_map
        .and_then(|map| map.get(key))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn classify_turn_failure(
    error_code: &str,
    detail_map: Option<&serde_json::Map<String, serde_json::Value>>,
) -> &'static str {
    if error_code == "E_PROVIDER_TOOL_SCHEMA" {
        return "provider_schema";
    }

    if let Some(category_hint) = detail_string(detail_map, "category_hint") {
        return match category_hint.as_str() {
            "tool_schema" => "provider_schema",
            "model_content" => "model_content",
            "rate_limit" => "provider_rate_limit",
            "auth" => "provider_auth",
            "server" => "provider_server",
            "network" => "provider_network",
            "context_limit" => "context_limit",
            "cancelled" => "cancelled",
            _ => "runtime",
        };
    }

    if error_code == "E_MODEL_CONTENT_REJECTED" {
        return "model_content";
    }
    if error_code == "E_RATE_LIMIT" {
        return "provider_rate_limit";
    }
    if error_code == "E_AUTH" {
        return "provider_auth";
    }
    if error_code == "E_SERVER_ERROR" {
        return "provider_server";
    }
    if error_code == "E_NETWORK" {
        return "provider_network";
    }
    if error_code == "E_CONTEXT_LIMIT" {
        return "context_limit";
    }
    if error_code == "E_CANCELLED" {
        return "cancelled";
    }
    if error_code.starts_with("E_TOOL_") {
        return "tool_execution";
    }

    "runtime"
}

fn is_provider_400_error(detail_map: Option<&serde_json::Map<String, serde_json::Value>>) -> bool {
    detail_map
        .and_then(|map| map.get("http_status"))
        .and_then(|value| value.as_i64())
        == Some(400)
}

fn is_missing_tool_escalation(
    error_code: &str,
    detail_map: Option<&serde_json::Map<String, serde_json::Value>>,
) -> bool {
    if matches!(error_code, "E_TOOL_NOT_FOUND" | "E_TOOL_SCHEMA_INVALID") {
        if let Some(diagnostic) = detail_string(detail_map, "diagnostic") {
            let normalized = diagnostic.to_ascii_lowercase();
            return normalized.contains("unknown tool")
                || normalized.contains("tool not found")
                || normalized.contains("missing tool");
        }

        return true;
    }

    false
}

fn emit_turn_failed_observability_metrics(
    session_id: &str,
    provider_name: &str,
    model_name: &str,
    telemetry: &super::tool_routing::ToolExposureTelemetry,
    error_code: &str,
    detail: Option<&serde_json::Value>,
    total_tool_calls: u32,
    rounds_executed: u32,
) {
    let detail_map = detail.and_then(|value| value.as_object());
    let classification = classify_turn_failure(error_code, detail_map);
    let provider_schema_error = classification == "provider_schema";
    let provider_400_error = is_provider_400_error(detail_map);
    let missing_tool_escalation = is_missing_tool_escalation(error_code, detail_map);
    let failed_before_first_token = rounds_executed <= 1 && total_tool_calls == 0;

    tracing::warn!(
        target: "agent_engine_observability",
        session_id = %session_id,
        provider = %provider_name,
        model = %model_name,
        tool_package = ?telemetry.tool_package,
        route_reason = %telemetry.route_reason,
        error_code = %error_code,
        turn_failed_classification = classification,
        tool_call_count = total_tool_calls,
        rounds_executed = rounds_executed,
        metric = "missing_tool_escalation_rate",
        value = if missing_tool_escalation { 1_u8 } else { 0_u8 },
        "turn failed telemetry captured"
    );

    if provider_schema_error {
        tracing::warn!(
            target: "agent_engine_observability",
            session_id = %session_id,
            provider = %provider_name,
            model = %model_name,
            metric = "tool_schema_reject_count",
            value = 1_u8,
            "provider rejected tool schema"
        );
    }

    if provider_400_error {
        tracing::warn!(
            target: "agent_engine_observability",
            session_id = %session_id,
            provider = %provider_name,
            model = %model_name,
            metric = "provider_400_count",
            value = 1_u8,
            "provider returned HTTP 400"
        );
    }

    if failed_before_first_token {
        tracing::warn!(
            target: "agent_engine_observability",
            session_id = %session_id,
            provider = %provider_name,
            model = %model_name,
            metric = "turn_failed_before_first_token_count",
            value = 1_u8,
            "turn failed before producing first token"
        );
    }
}

fn enrich_turn_failure_detail(
    detail: Option<serde_json::Value>,
    error_code: &str,
    provider_name: &str,
    model_name: &str,
    telemetry: &super::tool_routing::ToolExposureTelemetry,
    exposed_tools: &[String],
    skipped_tools: &[crate::agent_tools::registry::ToolSchemaSkipDiagnostic],
    total_tool_calls: u32,
    rounds_executed: u32,
) -> Option<serde_json::Value> {
    let mut detail = match detail {
        Some(serde_json::Value::Object(map)) => serde_json::Value::Object(map),
        Some(other) => json!({ "raw_detail": other }),
        None => json!({}),
    };

    let Some(map) = detail.as_object_mut() else {
        return Some(detail);
    };

    if !provider_name.trim().is_empty() && !map.contains_key("provider") {
        map.insert("provider".to_string(), json!(provider_name));
    }
    if !model_name.trim().is_empty() {
        map.insert("model".to_string(), json!(model_name));
    }
    map.insert("tool_package".to_string(), json!(telemetry.tool_package));
    map.insert("route_reason".to_string(), json!(telemetry.route_reason));
    map.insert("rollout_mode".to_string(), json!(telemetry.rollout_mode));
    map.insert(
        "rollout_in_canary".to_string(),
        json!(telemetry.rollout_in_canary),
    );
    if let Some(canary_percent) = telemetry.canary_percent {
        map.insert("canary_percent".to_string(), json!(canary_percent));
    }
    if let Some(fallback_from) = telemetry.fallback_from {
        map.insert("fallback_from".to_string(), json!(fallback_from));
    }
    if let Some(fallback_reason) = telemetry.fallback_reason.as_deref() {
        map.insert("fallback_reason".to_string(), json!(fallback_reason));
    }
    map.insert("exposed_tools".to_string(), json!(exposed_tools));
    if !skipped_tools.is_empty() {
        map.insert("skipped_tools".to_string(), json!(skipped_tools));
    }

    let detail_map = Some(&*map);
    let classification = classify_turn_failure(error_code, detail_map);
    let provider_schema_error = classification == "provider_schema";
    let provider_400_error = is_provider_400_error(detail_map);
    let missing_tool_escalation = is_missing_tool_escalation(error_code, detail_map);

    map.insert(
        "turn_failed_classification".to_string(),
        json!(classification),
    );
    map.insert(
        "provider_schema_error".to_string(),
        json!(provider_schema_error),
    );
    map.insert("provider_400_error".to_string(), json!(provider_400_error));
    map.insert(
        "missing_tool_escalation".to_string(),
        json!(missing_tool_escalation),
    );
    map.insert("tool_call_count".to_string(), json!(total_tool_calls));
    map.insert("rounds_executed".to_string(), json!(rounds_executed));
    map.insert(
        "fallback_occurred".to_string(),
        json!(telemetry.fallback_from.is_some()),
    );

    Some(detail)
}

/// After tool execution, check if a `skill` tool was called successfully and update active_skill.
fn update_active_skill_from_tool_results(
    tool_messages: &[AgentMessage],
    active_skill: &mut Option<String>,
) {
    for msg in tool_messages {
        for block in &msg.blocks {
            if let super::messages::ContentBlock::ToolResult {
                tool_name: Some(name),
                content,
                is_error: false,
                ..
            } = block
            {
                if name == "skill" {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
                        if let Some(skill_name) = data.get("skill_name").and_then(|v| v.as_str()) {
                            *active_skill = Some(skill_name.to_string());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::tool_routing::{ToolExposureTelemetry, ToolPackageName};
    use serde_json::json;

    fn make_tool_call(name: &str, args: serde_json::Value) -> ToolCallInfo {
        ToolCallInfo {
            llm_call_id: format!("call_{}", name),
            tool_name: name.to_string(),
            args,
        }
    }

    fn valid_todowrite_args() -> serde_json::Value {
        json!({
            "todos": [
                { "status": "pending", "text": "Step 1: implement feature" }
            ]
        })
    }

    // ── check_plan_before_write_policy tests ──

    #[test]
    fn policy_allows_todowrite_then_edit() {
        let calls = vec![
            make_tool_call("todowrite", valid_todowrite_args()),
            make_tool_call("edit", json!({"path": "foo.rs", "old": "a", "new": "b"})),
        ];
        assert!(check_plan_before_write_policy(&calls).is_none());
    }

    #[test]
    fn policy_allows_todowrite_then_create() {
        let calls = vec![
            make_tool_call("todowrite", valid_todowrite_args()),
            make_tool_call(
                "create",
                json!({"path": "new.rs", "content": "fn main() {}"}),
            ),
        ];
        assert!(check_plan_before_write_policy(&calls).is_none());
    }

    #[test]
    fn policy_allows_read_only_calls() {
        let calls = vec![
            make_tool_call("read", json!({"path": "foo.rs"})),
            make_tool_call("ls", json!({"path": "."})),
            make_tool_call("grep", json!({"pattern": "fn main"})),
        ];
        assert!(check_plan_before_write_policy(&calls).is_none());
    }

    #[test]
    fn policy_rejects_invalid_todowrite_before_edit() {
        // todowrite with missing 'todos' field is invalid
        let calls = vec![
            make_tool_call("todowrite", json!({"wrong_field": "oops"})),
            make_tool_call("edit", json!({"path": "foo.rs", "old": "a", "new": "b"})),
        ];
        let violation = check_plan_before_write_policy(&calls);
        assert!(violation.is_some());
        let v = violation.unwrap();
        assert_eq!(v.violating_tool, "edit");
        assert_eq!(v.violating_index, 1);
    }

    #[test]
    fn policy_empty_calls_is_ok() {
        assert!(check_plan_before_write_policy(&[]).is_none());
    }

    // ── is_write_tool tests ──

    #[test]
    fn write_tool_detection() {
        assert!(is_write_tool("edit"));
        assert!(is_write_tool("create"));
        assert!(!is_write_tool("read"));
        assert!(!is_write_tool("ls"));
        assert!(!is_write_tool("grep"));
        assert!(!is_write_tool("todowrite"));
    }

    // ── update_active_skill_from_tool_results tests ──

    #[test]
    fn update_skill_from_tool_result() {
        let msg = AgentMessage::tool_result(
            "call_skill".to_string(),
            Some("skill".to_string()),
            json!({"skill_name": "writing_assistant"}).to_string(),
            false,
        );
        let mut active_skill: Option<String> = None;
        update_active_skill_from_tool_results(&[msg], &mut active_skill);
        assert_eq!(active_skill, Some("writing_assistant".to_string()));
    }

    #[test]
    fn update_skill_ignores_error_results() {
        let msg = AgentMessage::tool_result(
            "call_skill".to_string(),
            Some("skill".to_string()),
            json!({"skill_name": "should_not_set"}).to_string(),
            true, // is_error
        );
        let mut active_skill: Option<String> = None;
        update_active_skill_from_tool_results(&[msg], &mut active_skill);
        assert_eq!(active_skill, None);
    }

    #[test]
    fn update_skill_ignores_non_skill_tools() {
        let msg = AgentMessage::tool_result(
            "call_read".to_string(),
            Some("read".to_string()),
            "file contents here".to_string(),
            false,
        );
        let mut active_skill = Some("existing_skill".to_string());
        update_active_skill_from_tool_results(&[msg], &mut active_skill);
        assert_eq!(active_skill, Some("existing_skill".to_string()));
    }

    #[test]
    fn update_skill_takes_last_skill_call() {
        let msg1 = AgentMessage::tool_result(
            "call_skill_1".to_string(),
            Some("skill".to_string()),
            json!({"skill_name": "first_skill"}).to_string(),
            false,
        );
        let msg2 = AgentMessage::tool_result(
            "call_skill_2".to_string(),
            Some("skill".to_string()),
            json!({"skill_name": "second_skill"}).to_string(),
            false,
        );
        let mut active_skill: Option<String> = None;
        update_active_skill_from_tool_results(&[msg1, msg2], &mut active_skill);
        assert_eq!(active_skill, Some("second_skill".to_string()));
    }

    #[test]
    fn enrich_turn_failure_detail_includes_provider_model_and_tool_exposure() {
        let detail = enrich_turn_failure_detail(
            Some(json!({
                "code": "E_PROVIDER_TOOL_SCHEMA",
                "diagnostic": "Invalid schema for function 'edit'"
            })),
            "E_PROVIDER_TOOL_SCHEMA",
            "openai-compatible",
            "gpt-4o-mini",
            &ToolExposureTelemetry {
                tool_package: ToolPackageName::Writing,
                route_reason: "writing_signal".to_string(),
                fallback_from: Some(ToolPackageName::LightChat),
                fallback_reason: Some("light_chat_to_writing".to_string()),
                rollout_mode: "canary".to_string(),
                rollout_in_canary: true,
                canary_percent: Some(10),
                exposed_tools: vec!["read".to_string(), "edit".to_string()],
                skipped_tools: vec![],
            },
            &["read".to_string(), "edit".to_string()],
            &[crate::agent_tools::registry::ToolSchemaSkipDiagnostic {
                tool_name: "skill".to_string(),
                error: "unsupported keyword 'const'".to_string(),
            }],
            0,
            1,
        )
        .expect("detail");

        assert_eq!(detail["provider"], "openai-compatible");
        assert_eq!(detail["model"], "gpt-4o-mini");
        assert_eq!(detail["tool_package"], "writing");
        assert_eq!(detail["fallback_from"], "light_chat");
        assert_eq!(detail["exposed_tools"][0], "read");
        assert_eq!(detail["skipped_tools"][0]["tool_name"], "skill");
        assert_eq!(detail["turn_failed_classification"], "provider_schema");
        assert_eq!(detail["provider_schema_error"], true);
        assert_eq!(detail["fallback_occurred"], true);
        assert_eq!(detail["tool_call_count"], 0);
    }
}
