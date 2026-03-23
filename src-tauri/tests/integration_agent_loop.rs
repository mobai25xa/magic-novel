/// Integration tests: mock LLM provider → AgentLoop → assert event sequence
///
/// Based on docs/magic_plan/plan_agent_parallel/supplement.md S5.1
///
/// These tests:
/// 1. Build a mock LlmProvider (returns pre-defined responses)
/// 2. Run AgentLoop end-to-end
/// 3. Assert emitted events (via a capturing EventSink)
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use magic_novel_lib::agent_engine::{
    emitter::EventSink,
    loop_engine::AgentLoop,
    messages::{AgentMessage, ContentBlock, ConversationState, Role},
    turn::{TurnEngine, TurnOutput},
    types::{ApprovalMode, ClarificationMode, LoopConfig, StopReason, ToolCallInfo, UsageInfo},
};
use magic_novel_lib::models::{AppError, ProjectMetadata, VolumeMetadata};
use serde::Serialize;
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

struct RecordingTurnEngine {
    responses: Mutex<std::collections::VecDeque<MockTurnStep>>,
    seen_tool_names: Arc<Mutex<Vec<Vec<String>>>>,
}

impl RecordingTurnEngine {
    fn new(responses: Vec<TurnOutput>) -> Self {
        Self {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(MockTurnStep::Output)
                    .collect::<std::collections::VecDeque<_>>(),
            ),
            seen_tool_names: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn recorded_tool_names(&self) -> Vec<Vec<String>> {
        self.seen_tool_names.lock().unwrap().clone()
    }
}

#[async_trait]
impl TurnEngine for RecordingTurnEngine {
    async fn execute_turn(
        &self,
        _state: &ConversationState,
        tool_schemas: &serde_json::Value,
    ) -> Result<TurnOutput, AppError> {
        self.seen_tool_names
            .lock()
            .unwrap()
            .push(schema_tool_names(tool_schemas));

        match self.responses.lock().unwrap().pop_front() {
            Some(MockTurnStep::Output(output)) => Ok(output),
            Some(MockTurnStep::Error(error)) => Err(error),
            None => Err(AppError::internal("RecordingTurnEngine: no more responses")),
        }
    }
}

struct TestProjectFixture {
    root: PathBuf,
    volume_id: String,
}

impl TestProjectFixture {
    fn new() -> Self {
        let root = std::env::current_dir()
            .expect("resolve current dir")
            .join("target")
            .join("agent_loop_test_projects")
            .join(format!("fixture_{}", uuid::Uuid::new_v4()));

        fs::create_dir_all(root.join("manuscripts").join("vol_1"))
            .expect("create project manuscripts");
        fs::create_dir_all(root.join(".magic_novel")).expect("create knowledge root");

        let project = ProjectMetadata::new(
            "integration-agent-loop".to_string(),
            "tester".to_string(),
            None,
            None,
        );
        write_json_file(&root.join("project.json"), &project);

        let mut volume = VolumeMetadata::new("卷一".to_string());
        volume.volume_id = "vol_1".to_string();
        write_json_file(
            &root.join("manuscripts").join("vol_1").join("volume.json"),
            &volume,
        );

        fs::write(
            root.join(".magic_novel").join("guidelines.md"),
            "# Writing Rules\n\nKeep the voice consistent.\n",
        )
        .expect("write guidelines");

        Self {
            root,
            volume_id: volume.volume_id,
        }
    }

    fn project_path(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    fn volume_ref(&self) -> String {
        format!("volume:manuscripts/{}/volume.json", self.volume_id)
    }
}

impl Drop for TestProjectFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn write_json_file<T: Serialize>(path: &Path, value: &T) {
    let payload = serde_json::to_vec_pretty(value).expect("serialize json");
    fs::write(path, payload).expect("write json file");
}

fn schema_tool_names(tool_schemas: &serde_json::Value) -> Vec<String> {
    tool_schemas
        .as_array()
        .expect("tool schemas should be an array")
        .iter()
        .filter_map(|tool| {
            tool.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|name| name.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn system_messages(state: &ConversationState) -> Vec<String> {
    state
        .messages
        .iter()
        .filter(|message| message.role == Role::System)
        .map(AgentMessage::text_content)
        .collect()
}

fn tool_result_contents(state: &ConversationState) -> Vec<String> {
    state
        .messages
        .iter()
        .filter(|message| message.role == Role::Tool)
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolResult { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect()
}

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

    // Round 1: LLM returns a tool call (read-only)
    // Round 2: LLM returns text (no more tool calls → done)
    let mock = MockTurnEngine::new(vec![
        tool_turn("workspace_map", json!({})),
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

    assert_eq!(plan_payload["tool_package"], "main_agent_core");
    assert!(plan_payload["fallback_from"].is_null());
    assert!(plan_payload["exposed_tools"]
        .as_array()
        .expect("exposed_tools array")
        .iter()
        .any(|tool| tool == "draft_write"));
    assert_eq!(completed_payload["tool_package"], "main_agent_core");
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
        tool_turn("workspace_map", json!({})),
        tool_turn("workspace_map", json!({})),
    ]);

    let config = LoopConfig {
        max_rounds: 10,
        max_tool_calls: 1,
        approval_mode: ApprovalMode::Auto,
        clarification_mode: ClarificationMode::Interactive,
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
        .map(|_| tool_turn("workspace_map", json!({})))
        .collect();

    let mock = MockTurnEngine::new(responses);

    let config = LoopConfig {
        max_rounds: 3, // low limit
        max_tool_calls: 100,
        approval_mode: ApprovalMode::Auto,
        clarification_mode: ClarificationMode::Interactive,
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
            "draft_write",
            json!({
                "target_ref": "chapter:manuscripts/vol_1/ch_1.json",
                "write_mode": "rewrite",
                "instruction": "Update chapter",
                "content": { "kind": "markdown", "value": "# Updated" },
                "dry_run": false,
            }),
        ),
    ])]);

    let config = LoopConfig {
        max_rounds: 5,
        max_tool_calls: 50,
        approval_mode: ApprovalMode::ConfirmWrites,
        clarification_mode: ClarificationMode::Interactive,
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
async fn test_todowrite_unknown_worker_field_surfaces_tool_error() {
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = MockTurnEngine::new(vec![
        tool_turn(
            "todowrite",
            json!({
                "todos": [
                    { "status": "pending", "text": "Plan outline", "worker": "plot-architect" }
                ]
            }),
        ),
        text_turn("Checklist recorded."),
    ]);

    let agent_loop = AgentLoop::new(
        sink.clone(),
        default_loop_config(),
        "/tmp/test_project".to_string(),
        cancel,
    );

    let mut conv = ConversationState::new("sess_todowrite_unknown_worker".to_string());
    conv.messages
        .push(AgentMessage::user("Track this task".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();
    assert_eq!(result.stop_reason, StopReason::Success);
    assert_eq!(result.rounds_executed, 2);
    assert_eq!(result.total_tool_calls, 1);

    let tool_contents = tool_result_contents(&conv);
    assert!(
        tool_contents
            .iter()
            .any(|content| content.contains("unknown field") && content.contains("worker")),
        "expected todowrite tool error in tool result content: {tool_contents:?}"
    );
    assert!(
        !tool_contents
            .iter()
            .any(|content| content.contains("Plan outline")),
        "unknown worker field should not be accepted as a persisted todo item: {tool_contents:?}"
    );

    let event_types = sink.event_types();
    assert!(
        !event_types.contains(&"TURN_FAILED".to_string()),
        "tool parse errors should stay inside the tool result contract: {event_types:?}"
    );
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

#[tokio::test]
async fn test_main_session_core_tool_does_not_surface_tool_not_allowed() {
    let fixture = TestProjectFixture::new();
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = RecordingTurnEngine::new(vec![
        tool_turn(
            "structure_edit",
            json!({
                "op": "create",
                "node_type": "chapter",
                "parent_ref": fixture.volume_ref(),
                "title": "第一章",
                "dry_run": true,
            }),
        ),
        text_turn("Done."),
    ]);

    let agent_loop = AgentLoop::new(sink, default_loop_config(), fixture.project_path(), cancel);

    let mut conv = ConversationState::new("sess_main_core_tool".to_string());
    conv.messages
        .push(AgentMessage::user("请直接创建第一章".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Success);

    let recorded = mock.recorded_tool_names();
    assert!(recorded.len() >= 2, "expected two rounds, got {recorded:?}");
    assert!(recorded[0].contains(&"structure_edit".to_string()));
    assert!(recorded[1].contains(&"structure_edit".to_string()));

    let tool_contents = tool_result_contents(&conv);
    assert!(
        tool_contents
            .iter()
            .all(|content| !content.contains("E_TOOL_NOT_ALLOWED")),
        "unexpected tool_not_allowed surfaced in tool results: {tool_contents:?}"
    );
}

#[tokio::test]
async fn test_structure_failure_keeps_structure_edit_visible_next_round() {
    let fixture = TestProjectFixture::new();
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = RecordingTurnEngine::new(vec![
        tool_turn(
            "structure_edit",
            json!({
                "op": "create",
                "node_type": "chapter",
                "parent_ref": "volume:manuscripts/missing_volume/volume.json",
                "title": "第一章",
                "dry_run": true,
            }),
        ),
        text_turn("Recovered."),
    ]);

    let agent_loop = AgentLoop::new(sink, default_loop_config(), fixture.project_path(), cancel);

    let mut conv = ConversationState::new("sess_structure_recovery".to_string());
    conv.messages
        .push(AgentMessage::user("创建缺失章节".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Success);

    let recorded = mock.recorded_tool_names();
    assert!(recorded.len() >= 2, "expected two rounds, got {recorded:?}");
    assert!(recorded[1].contains(&"structure_edit".to_string()));

    let tool_contents = tool_result_contents(&conv);
    assert!(
        tool_contents
            .iter()
            .any(|content| content.contains("E_REF_NOT_FOUND")),
        "expected missing-structure error in tool results: {tool_contents:?}"
    );

    let system_texts = system_messages(&conv);
    assert!(
        system_texts
            .iter()
            .any(|text| text.contains("System recovery note")),
        "expected recovery note after structure failure: {system_texts:?}"
    );
    assert!(
        system_texts
            .iter()
            .any(|text| text.contains("Use `workspace_map` to inspect refs, then `structure_edit`")),
        "expected structure recovery guidance: {system_texts:?}"
    );
}

#[tokio::test]
async fn test_draft_write_failure_keeps_draft_write_and_structure_edit_visible_next_round() {
    let fixture = TestProjectFixture::new();
    let sink = CapturingEventSink::new();
    let cancel = CancellationToken::new();

    let mock = RecordingTurnEngine::new(vec![
        tool_turn(
            "draft_write",
            json!({
                "target_ref": "chapter:manuscripts/vol_1/ch_missing.json",
                "write_mode": "rewrite",
                "instruction": "Write the opening scene",
                "content": { "kind": "markdown", "value": "# 第一章\n\n开场。" },
                "dry_run": true,
            }),
        ),
        text_turn("Recovered."),
    ]);

    let agent_loop = AgentLoop::new(sink, default_loop_config(), fixture.project_path(), cancel);

    let mut conv = ConversationState::new("sess_draft_write_recovery".to_string());
    conv.messages
        .push(AgentMessage::user("直接写第一章".to_string()));

    let result = agent_loop.run(&mut conv, &mock).await.unwrap();

    assert_eq!(result.stop_reason, StopReason::Success);

    let recorded = mock.recorded_tool_names();
    assert!(recorded.len() >= 2, "expected two rounds, got {recorded:?}");
    assert!(recorded[1].contains(&"draft_write".to_string()));
    assert!(recorded[1].contains(&"structure_edit".to_string()));

    let tool_contents = tool_result_contents(&conv);
    assert!(
        tool_contents
            .iter()
            .any(|content| content.contains("E_REF_NOT_FOUND")),
        "expected missing-chapter error in tool results: {tool_contents:?}"
    );

    let system_texts = system_messages(&conv);
    assert!(
        system_texts
            .iter()
            .any(|text| text.contains("Use `workspace_map` to inspect refs, then `structure_edit`")),
        "expected recovery guidance after draft_write failure: {system_texts:?}"
    );
}
