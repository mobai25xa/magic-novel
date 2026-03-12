/// Integration tests: mock LLM provider → AgentLoop → assert event sequence
///
/// Based on docs/magic_plan/plan_agent_parallel/supplement.md S5.1
///
/// These tests:
/// 1. Build a mock LlmProvider (returns pre-defined responses)
/// 2. Run AgentLoop end-to-end
/// 3. Assert emitted events (via a capturing EventSink)
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use magic_novel_lib::agent_engine::{
    emitter::EventSink,
    loop_engine::AgentLoop,
    messages::{AgentMessage, ConversationState},
    turn::{TurnEngine, TurnOutput},
    types::{ApprovalMode, ClarificationMode, LoopConfig, StopReason, ToolCallInfo, UsageInfo},
};
use magic_novel_lib::models::AppError;
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ── Capturing EventSink ──────────────────────────────────────────

/// Records all emitted (event_type, payload) pairs for assertions.
#[derive(Clone)]
struct CapturingEventSink {
    events: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

impl CapturingEventSink {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn recorded(&self) -> Vec<(String, serde_json::Value)> {
        self.events.lock().unwrap().clone()
    }

    fn event_types(&self) -> Vec<String> {
        self.recorded().into_iter().map(|(t, _)| t).collect()
    }
}

impl EventSink for CapturingEventSink {
    fn emit_raw(&self, event_type: &str, payload: serde_json::Value) -> Result<(), AppError> {
        self.events
            .lock()
            .unwrap()
            .push((event_type.to_string(), payload));
        Ok(())
    }
}

// ── Mock TurnEngine ──────────────────────────────────────────────

/// Returns a fixed sequence of TurnOutput or AppError values, one per call.
enum MockTurnStep {
    Output(TurnOutput),
    Error(AppError),
}

struct MockTurnEngine {
    responses: Mutex<std::collections::VecDeque<MockTurnStep>>,
}

impl MockTurnEngine {
    fn new(responses: Vec<TurnOutput>) -> Self {
        Self::from_steps(responses.into_iter().map(MockTurnStep::Output).collect())
    }

    fn from_steps(steps: Vec<MockTurnStep>) -> Self {
        Self {
            responses: Mutex::new(steps.into_iter().collect()),
        }
    }
}

#[async_trait]
impl TurnEngine for MockTurnEngine {
    async fn execute_turn(
        &self,
        _state: &ConversationState,
        _tool_schemas: &serde_json::Value,
    ) -> Result<TurnOutput, AppError> {
        match self.responses.lock().unwrap().pop_front() {
            Some(MockTurnStep::Output(output)) => Ok(output),
            Some(MockTurnStep::Error(error)) => Err(error),
            None => Err(AppError::internal("MockTurnEngine: no more responses")),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn text_turn(text: &str) -> TurnOutput {
    TurnOutput {
        assistant_message: AgentMessage::system(text.to_string()),
        tool_calls: vec![],
        stop_reason: StopReason::Success,
        usage: Some(UsageInfo {
            input_tokens: 10,
            output_tokens: 5,
            cache_read_tokens: 0,
            thinking_tokens: 0,
        }),
    }
}

fn tool_turns(calls: Vec<(&str, serde_json::Value)>) -> TurnOutput {
    use magic_novel_lib::agent_engine::messages::{ContentBlock, Role};

    let mut tool_calls = Vec::new();
    let mut blocks = Vec::new();

    for (idx, (tool_name, args)) in calls.into_iter().enumerate() {
        let tc = ToolCallInfo {
            llm_call_id: format!("call_{}_{}", tool_name, idx),
            tool_name: tool_name.to_string(),
            args,
        };

        blocks.push(ContentBlock::ToolCall {
            id: tc.llm_call_id.clone(),
            name: tc.tool_name.clone(),
            input: tc.args.clone(),
        });
        tool_calls.push(tc);
    }

    TurnOutput {
        assistant_message: AgentMessage {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            role: Role::Assistant,
            blocks,
            ts: chrono::Utc::now().timestamp_millis(),
        },
        tool_calls,
        stop_reason: StopReason::Success,
        usage: None,
    }
}

fn tool_turn(tool_name: &str, args: serde_json::Value) -> TurnOutput {
    tool_turns(vec![(tool_name, args)])
}

fn default_loop_config() -> LoopConfig {
    LoopConfig {
        max_rounds: 10,
        max_tool_calls: 50,
        approval_mode: ApprovalMode::Auto,
        clarification_mode: ClarificationMode::Interactive,
        worker_dispatch_enabled: false,
        ..LoopConfig::default()
    }
}

fn context_limit_error() -> AppError {
    AppError {
        code: magic_novel_lib::models::ErrorCode::Internal,
        message: "context_length_exceeded".to_string(),
        details: Some(json!({ "code": "E_CONTEXT_LIMIT" })),
        recoverable: Some(true),
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_text_only_turn_emits_started_and_completed() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    // Emit turn_started manually (as agent_turn_start would)
    use magic_novel_lib::agent_engine::emitter::EventSink;
    sink.turn_started("hello world", "openai", "gpt-4o")
        .unwrap();

    let agent_loop = AgentLoop::new(
        sink.clone(),
        default_loop_config(),
        "/tmp/test_project".to_string(),
        cancel,
    );

    let mock = MockTurnEngine::new(vec![text_turn("All done!")]);
    let mut conv = ConversationState::new("sess_test_1".to_string());
    conv.messages
        .push(AgentMessage::user("Do something".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Success);
    assert_eq!(result.rounds_executed, 1);
    assert_eq!(result.total_tool_calls, 0);

    let types = sink.event_types();
    assert!(
        types.contains(&"TURN_STARTED".to_string()),
        "expected TURN_STARTED, got {types:?}"
    );
    assert!(
        types.contains(&"USAGE_UPDATE".to_string()),
        "expected USAGE_UPDATE, got {types:?}"
    );
    assert!(
        types.contains(&"TURN_COMPLETED".to_string()),
        "expected TURN_COMPLETED, got {types:?}"
    );
}

#[tokio::test]
async fn test_tool_call_turn_emits_tool_events() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    // Round 1: LLM returns a tool call (read)
    // Round 2: LLM returns text (no more tool calls → done)
    let mock = MockTurnEngine::new(vec![
        tool_turn(
            "read",
            json!({ "path": "manuscripts/vol_1/ch_1.md", "view": "markdown" }),
        ),
        text_turn("I read the chapter."),
    ]);

    let agent_loop = AgentLoop::new(
        sink.clone(),
        default_loop_config(),
        "/tmp/test_project".to_string(),
        cancel,
    );

    let mut conv = ConversationState::new("sess_test_2".to_string());
    conv.messages
        .push(AgentMessage::user("List the files".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.rounds_executed, 2);
    assert_eq!(result.total_tool_calls, 1);

    let types = sink.event_types();
    assert!(
        types.contains(&"TOOL_CALL_STARTED".to_string()),
        "expected TOOL_CALL_STARTED, got {types:?}"
    );
    assert!(
        types.contains(&"TOOL_CALL_FINISHED".to_string()),
        "expected TOOL_CALL_FINISHED, got {types:?}"
    );
    assert!(
        types.contains(&"TURN_COMPLETED".to_string()),
        "expected TURN_COMPLETED, got {types:?}"
    );
}

#[tokio::test]
async fn test_loop_emits_tool_package_telemetry() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = MockTurnEngine::new(vec![text_turn("Rewritten.")]);

    let agent_loop = AgentLoop::new(
        sink.clone(),
        default_loop_config(),
        "/tmp/test_project".to_string(),
        cancel,
    );

    let mut conv = ConversationState::new("sess_test_package".to_string());
    conv.messages.push(AgentMessage::user(
        "Please rewrite chapter 3 in a tighter voice".to_string(),
    ));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();
    assert_eq!(result.stop_reason, StopReason::Success);

    let recorded = sink.recorded();
    let plan_payload = recorded
        .iter()
        .find(|(event_type, _)| event_type == "PLAN_STARTED")
        .map(|(_, payload)| payload.clone())
        .expect("expected PLAN_STARTED payload");
    let completed_payload = recorded
        .iter()
        .find(|(event_type, _)| event_type == "TURN_COMPLETED")
        .map(|(_, payload)| payload.clone())
        .expect("expected TURN_COMPLETED payload");

    assert_eq!(plan_payload["tool_package"], "writing");
    assert_eq!(plan_payload["fallback_from"], "light_chat");
    assert!(plan_payload["exposed_tools"]
        .as_array()
        .expect("exposed_tools array")
        .iter()
        .any(|tool| tool == "edit"));
    assert_eq!(completed_payload["tool_package"], "writing");
}

#[tokio::test]
async fn test_cancellation_stops_loop() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    // Cancel before the loop even starts
    cancel.cancel();

    let mock = MockTurnEngine::new(vec![text_turn("Should not reach")]);

    let agent_loop = AgentLoop::new(
        sink.clone(),
        default_loop_config(),
        "/tmp/test_project".to_string(),
        cancel,
    );

    let mut conv = ConversationState::new("sess_test_cancel".to_string());
    conv.messages.push(AgentMessage::user("Run".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Cancel);
    // Loop should have emitted TURN_CANCELLED
    let types = sink.event_types();
    assert!(
        types.contains(&"TURN_CANCELLED".to_string()),
        "expected TURN_CANCELLED, got {types:?}"
    );
}

#[tokio::test]
async fn test_max_tool_calls_safety_valve() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = MockTurnEngine::new(vec![
        tool_turn("ls", json!({ "cwd": "/tmp", "project_path": "/tmp" })),
        tool_turn("ls", json!({ "cwd": "/tmp", "project_path": "/tmp" })),
    ]);

    let config = LoopConfig {
        max_rounds: 10,
        max_tool_calls: 1,
        approval_mode: ApprovalMode::Auto,
        clarification_mode: ClarificationMode::Interactive,
        worker_dispatch_enabled: false,
        ..LoopConfig::default()
    };

    let agent_loop = AgentLoop::new(sink.clone(), config, "/tmp".to_string(), cancel);

    let mut conv = ConversationState::new("sess_test_maxtools".to_string());
    conv.messages
        .push(AgentMessage::user("Loop forever".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Limit);
    assert_eq!(result.total_tool_calls, 1);

    let events = sink.recorded();
    let turn_completed = events
        .iter()
        .find(|(event_type, _)| event_type == "TURN_COMPLETED")
        .expect("TURN_COMPLETED should be emitted on max_tool_calls limit");
    assert_eq!(turn_completed.1["stop_reason"], "limit");
}

#[tokio::test]
async fn test_max_rounds_safety_valve() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    // Always return a tool call → loop will hit max_rounds
    let responses: Vec<TurnOutput> = (0..20)
        .map(|_| tool_turn("ls", json!({ "cwd": "/tmp", "project_path": "/tmp" })))
        .collect();

    let mock = MockTurnEngine::new(responses);

    let config = LoopConfig {
        max_rounds: 3, // low limit
        max_tool_calls: 100,
        approval_mode: ApprovalMode::Auto,
        clarification_mode: ClarificationMode::Interactive,
        worker_dispatch_enabled: false,
        ..LoopConfig::default()
    };

    let agent_loop = AgentLoop::new(sink.clone(), config, "/tmp".to_string(), cancel);

    let mut conv = ConversationState::new("sess_test_maxrounds".to_string());
    conv.messages
        .push(AgentMessage::user("Loop forever".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Limit);
    assert_eq!(result.rounds_executed, 3);
}

#[tokio::test]
async fn test_waiting_confirmation_turn_completes_with_wait_reason() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = MockTurnEngine::new(vec![tool_turns(vec![
        (
            "todowrite",
            json!({
                "todos": [
                    { "status": "in_progress", "text": "Update chapter title" }
                ]
            }),
        ),
        (
            "edit",
            json!({
                "path": "manuscripts/vol_1/ch_1.md",
                "content": "# Updated",
                "dry_run": false,
            }),
        ),
    ])]);

    let config = LoopConfig {
        max_rounds: 5,
        max_tool_calls: 50,
        approval_mode: ApprovalMode::ConfirmWrites,
        clarification_mode: ClarificationMode::Interactive,
        worker_dispatch_enabled: false,
        ..LoopConfig::default()
    };

    let agent_loop = AgentLoop::new(
        sink.clone(),
        config,
        "/tmp/test_project".to_string(),
        cancel,
    );

    let mut conv = ConversationState::new("sess_wait_confirm".to_string());
    conv.messages
        .push(AgentMessage::user("Please edit chapter".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::WaitingConfirmation);

    let events = sink.recorded();
    let turn_completed = events
        .iter()
        .find(|(event_type, _)| event_type == "TURN_COMPLETED")
        .expect("TURN_COMPLETED should be emitted on waiting confirmation");

    assert_eq!(turn_completed.1["stop_reason"], "waiting_confirmation");
    assert_eq!(turn_completed.1["compacted"], false);
}

#[tokio::test]
async fn test_waiting_askuser_turn_completes_with_wait_reason() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = MockTurnEngine::new(vec![tool_turn(
        "askuser",
        json!({
            "questionnaire": "1. [question] Confirm rewrite?\n[topic] Rewrite\n[option] Yes\n[option] No"
        }),
    )]);

    let config = LoopConfig {
        max_rounds: 5,
        max_tool_calls: 50,
        approval_mode: ApprovalMode::ConfirmWrites,
        clarification_mode: ClarificationMode::Interactive,
        worker_dispatch_enabled: false,
        ..LoopConfig::default()
    };

    let agent_loop = AgentLoop::new(
        sink.clone(),
        config,
        "/tmp/test_project".to_string(),
        cancel,
    );

    let mut conv = ConversationState::new("sess_wait_askuser".to_string());
    conv.messages
        .push(AgentMessage::user("Need clarification".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::WaitingAskuser);

    let events = sink.recorded();
    let turn_completed = events
        .iter()
        .find(|(event_type, _)| event_type == "TURN_COMPLETED")
        .expect("TURN_COMPLETED should be emitted on waiting askuser");

    assert_eq!(turn_completed.1["stop_reason"], "waiting_askuser");
    assert_eq!(turn_completed.1["compacted"], false);
}

#[tokio::test]
async fn test_context_limit_compaction_retries_and_completes() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = MockTurnEngine::from_steps(vec![
        MockTurnStep::Error(context_limit_error()),
        MockTurnStep::Output(text_turn("Recovered after compaction")),
    ]);

    let mut conv = ConversationState::new("sess_context_retry".to_string());
    conv.messages
        .push(AgentMessage::system("You are assistant".to_string()));
    for i in 0..16 {
        conv.messages.push(AgentMessage::user(format!(
            "Long context part {i}: {}",
            "x".repeat(24_000)
        )));
    }

    let agent_loop = AgentLoop::new(
        sink.clone(),
        default_loop_config(),
        "/tmp/test_project".to_string(),
        cancel,
    )
    .with_provider_info(
        "openai-compatible".to_string(),
        "gpt-4o-mini".to_string(),
        String::new(),
        String::new(),
    );

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Success);

    let events = sink.recorded();
    assert!(
        events
            .iter()
            .any(|(event_type, _)| event_type == "COMPACTION_STARTED"),
        "expected COMPACTION_STARTED after context limit"
    );
    assert!(
        events
            .iter()
            .any(|(event_type, _)| event_type == "COMPACTION_FINISHED"),
        "expected COMPACTION_FINISHED after context limit"
    );

    let turn_completed = events
        .iter()
        .find(|(event_type, _)| event_type == "TURN_COMPLETED")
        .expect("TURN_COMPLETED should be emitted after retry success");
    assert_eq!(turn_completed.1["stop_reason"], "success");
    assert_eq!(conv.last_compaction.is_some(), true);
}

#[tokio::test]
async fn test_capturing_sink_emit_raw() {
    let sink = CapturingEventSink::new();
    use magic_novel_lib::agent_engine::emitter::EventSink;
    sink.emit_raw("TEST_EVENT", json!({ "foo": "bar" }))
        .unwrap();

    let events = sink.recorded();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "TEST_EVENT");
    assert_eq!(events[0].1["foo"], "bar");
}
