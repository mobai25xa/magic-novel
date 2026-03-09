use super::*;
use crate::agent_engine::messages::Role;
use crate::agent_engine::types::DEFAULT_MODEL;

fn setup_temp_project() -> std::path::PathBuf {
    let base =
        std::env::temp_dir().join(format!("magic_agent_engine_test_{}", uuid::Uuid::new_v4()));
    let sessions_dir = base.join("magic_novel").join("ai").join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("create sessions dir");
    base
}

#[test]
fn apply_system_prompt_inserts_default_for_empty_state() {
    let mut state = ConversationState::new("s1".to_string());
    apply_system_prompt(&mut state, None);

    assert_eq!(state.messages.len(), 1);
    assert!(matches!(state.messages[0].role, Role::System));
    assert_eq!(state.messages[0].text_content(), default_system_prompt());
    assert!(state.messages[0]
        .text_content()
        .contains("You are the Magic Novel AI writing assistant."));
}

#[test]
fn default_system_prompt_contains_edit_workflow_section() {
    let prompt = default_system_prompt();
    assert!(prompt.contains("## Edit workflow (critical)"));
    assert!(prompt.contains("base_revision"));
}

#[test]
fn default_system_prompt_contains_todowrite_milestone_policy() {
    let prompt = default_system_prompt();
    assert!(prompt.contains("For multi-step tasks, call todowrite at milestone boundaries"));
    assert!(prompt.contains("Keep todowrite entries user-verifiable only"));
    assert!(prompt.contains("For simple single-step tasks, skip todowrite and execute directly"));
}

#[test]
fn default_system_prompt_contains_all_tools() {
    let prompt = default_system_prompt();
    for tool in [
        "read",
        "edit",
        "create",
        "delete",
        "move",
        "ls",
        "grep",
        "outline",
        "character_sheet",
        "search_knowledge",
        "askuser",
        "todowrite",
    ] {
        assert!(
            prompt.contains(&format!("| {} |", tool)),
            "missing tool '{}' in default prompt",
            tool
        );
    }
}

#[test]
fn apply_system_prompt_updates_existing_system_message() {
    let mut state = ConversationState::new("s1".to_string());
    state
        .messages
        .push(AgentMessage::system("old system".to_string()));
    state.messages.push(AgentMessage::user("hello".to_string()));

    apply_system_prompt(&mut state, Some("new system"));

    assert_eq!(state.messages.len(), 2);
    assert!(matches!(state.messages[0].role, Role::System));
    assert_eq!(state.messages[0].text_content(), "new system");
    assert!(matches!(state.messages[1].role, Role::User));
}

#[test]
fn apply_system_prompt_preserves_existing_when_no_new_input() {
    let mut state = ConversationState::new("s1".to_string());
    state
        .messages
        .push(AgentMessage::system("existing system".to_string()));
    state.messages.push(AgentMessage::user("hello".to_string()));

    apply_system_prompt(&mut state, None);

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].text_content(), "existing system");
}

#[test]
fn resolve_turn_provider_config_uses_default_model_constant() {
    let input = AgentTurnStartInput {
        session_id: "s1".to_string(),
        client_request_id: None,
        user_text: "hello".to_string(),
        project_path: "D:/tmp/project".to_string(),
        model: None,
        provider: None,
        base_url: Some("https://example.com/v1".to_string()),
        api_key: Some("key".to_string()),
        system_prompt: None,
        active_chapter_path: None,
        active_skill: None,
        capability_mode: None,
        approval_mode: None,
        clarification_mode: None,
        editor_state: None,
    };

    let (_, _, model) = resolve_turn_provider_config(&input).expect("config should resolve");
    assert_eq!(model, DEFAULT_MODEL);
}

#[test]
fn build_loop_config_defaults_match_fix_mode_contract() {
    let input = AgentTurnStartInput {
        session_id: "s1".to_string(),
        client_request_id: None,
        user_text: "hello".to_string(),
        project_path: "D:/tmp/project".to_string(),
        model: None,
        provider: None,
        base_url: Some("https://example.com/v1".to_string()),
        api_key: Some("key".to_string()),
        system_prompt: None,
        active_chapter_path: None,
        active_skill: None,
        capability_mode: None,
        approval_mode: None,
        clarification_mode: None,
        editor_state: None,
    };

    let loop_config = build_loop_config(&input);
    assert_eq!(loop_config.capability_mode, AgentMode::Writing);
    assert_eq!(loop_config.approval_mode, ApprovalMode::ConfirmWrites);
    assert_eq!(
        loop_config.clarification_mode,
        ClarificationMode::Interactive
    );
    assert_eq!(
        loop_config.autonomy_level,
        ApprovalMode::ConfirmWrites.to_autonomy_level()
    );
}

#[test]
fn build_loop_config_respects_explicit_modes() {
    let input = AgentTurnStartInput {
        session_id: "s1".to_string(),
        client_request_id: None,
        user_text: "hello".to_string(),
        project_path: "D:/tmp/project".to_string(),
        model: None,
        provider: None,
        base_url: Some("https://example.com/v1".to_string()),
        api_key: Some("key".to_string()),
        system_prompt: None,
        active_chapter_path: None,
        active_skill: None,
        capability_mode: Some(AgentMode::Planning),
        approval_mode: Some(ApprovalMode::Auto),
        clarification_mode: Some(ClarificationMode::HeadlessDefer),
        editor_state: None,
    };

    let loop_config = build_loop_config(&input);
    assert_eq!(loop_config.capability_mode, AgentMode::Planning);
    assert_eq!(loop_config.approval_mode, ApprovalMode::Auto);
    assert_eq!(
        loop_config.clarification_mode,
        ClarificationMode::HeadlessDefer
    );
    assert_eq!(
        loop_config.autonomy_level,
        ApprovalMode::Auto.to_autonomy_level()
    );
}

#[test]
fn hydrate_existing_session_on_start_allows_new_session_without_runtime_snapshot() {
    let project = setup_temp_project();
    let session_id = format!("test_new_session_runtime_{}", uuid::Uuid::new_v4());

    let stream_path = crate::services::session_stream_path(project.as_path(), &session_id);
    let start_event = crate::services::AgentSessionEvent {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        event_type: "session_start".to_string(),
        session_id: session_id.clone(),
        ts: chrono::Utc::now().timestamp_millis(),
        event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
        event_seq: Some(1),
        dedupe_key: Some("session_start".to_string()),
        turn: None,
        payload: Some(serde_json::json!({"test": true})),
    };
    crate::services::append_events_jsonl(&stream_path, &[start_event]).expect("append event");

    let now = chrono::Utc::now().timestamp_millis();
    let meta = crate::services::AgentSessionMeta {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        session_id: session_id.clone(),
        title: Some("new session".to_string()),
        created_at: now,
        updated_at: now,
        last_turn: None,
        last_stop_reason: None,
        active_chapter_path: None,
        compaction_count: Some(0),
    };
    crate::application::command_usecases::agent_session_support::save_session_meta(
        project.as_path(),
        meta,
    )
    .expect("save meta");

    let hydrated =
        hydrate_existing_session_on_start(project.to_string_lossy().as_ref(), &session_id)
            .expect("new session should hydrate as empty conversation");

    assert_eq!(hydrated.conversation.session_id, session_id);
    assert_eq!(hydrated.conversation.current_turn, 0);
    assert_eq!(hydrated.next_turn_id, 1);

    crate::agent_engine::session_state::global().remove_session(&session_id);
}

#[test]
fn prepare_turn_start_restores_authoritative_turn_after_hydrate() {
    let project = setup_temp_project();
    let session_id = format!("test_prepare_start_restore_{}", uuid::Uuid::new_v4());

    let stream_path = crate::services::session_stream_path(project.as_path(), &session_id);
    let event = crate::services::AgentSessionEvent {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        event_type: "turn_started".to_string(),
        session_id: session_id.clone(),
        ts: chrono::Utc::now().timestamp_millis(),
        event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
        event_seq: Some(1),
        dedupe_key: None,
        turn: Some(7),
        payload: Some(serde_json::json!({"test": true})),
    };
    crate::services::append_events_jsonl(&stream_path, &[event]).expect("append event");

    let now = chrono::Utc::now().timestamp_millis();
    let meta = crate::services::AgentSessionMeta {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        session_id: session_id.clone(),
        title: Some("restore me".to_string()),
        created_at: now,
        updated_at: now,
        last_turn: Some(7),
        last_stop_reason: Some("success".to_string()),
        active_chapter_path: None,
        compaction_count: Some(0),
    };
    crate::application::command_usecases::agent_session_support::save_session_meta(
        project.as_path(),
        meta,
    )
    .expect("save meta");

    let mut conversation = ConversationState::new(session_id.clone());
    conversation.current_turn = 7;
    conversation
        .messages
        .push(AgentMessage::user("persisted user".to_string()));

    crate::services::save_runtime_snapshot_from_input(
        project.as_path(),
        RuntimeSnapshotUpsertInput::from_conversation(
            session_id.clone(),
            SessionRuntimeState::Completed,
            conversation,
            Some(7),
            Some("openai-compatible".to_string()),
            Some("gpt-4o-mini".to_string()),
            Some("https://api.openai.com/v1".to_string()),
            None,
            None,
            Some(LoopConfig::default()),
        )
        .with_active_skill(Some("story-architect".to_string())),
    )
    .expect("save runtime snapshot");

    let prepared = prepare_turn_start(
        project.to_string_lossy().as_ref(),
        &session_id,
        "client_req_restore",
    )
    .expect("prepare turn start");

    assert_eq!(prepared.turn_id, 8);
    assert_eq!(prepared.hydration_status, "snapshot_loaded");
    assert_eq!(prepared.active_skill.as_deref(), Some("story-architect"));

    crate::agent_engine::session_state::global().clear_cancel_token(&session_id);
    crate::agent_engine::session_state::global().remove_session(&session_id);
}

#[test]
fn persist_runtime_after_loop_preserves_suspended_state_for_waiting_askuser() {
    let project = setup_temp_project();
    let session_id = format!("test_preserve_suspend_{}", uuid::Uuid::new_v4());

    let mut conversation = ConversationState::new(session_id.clone());
    conversation.current_turn = 3;

    session_state::global().save_suspended_runtime_state(
        &session_id,
        session_state::SuspendedTurnState {
            conversation_state: conversation.clone(),
            pending_tool_call: crate::agent_engine::types::ToolCallInfo {
                llm_call_id: "call_ask_1".to_string(),
                tool_name: "askuser".to_string(),
                args: serde_json::json!({
                    "questions": [
                        {
                            "question": "继续吗?",
                            "topic": "continue",
                            "options": ["是", "否"]
                        }
                    ]
                }),
            },
            pending_call_id: "pending_ask_1".to_string(),
            remaining_tool_calls: Vec::new(),
            completed_messages: Vec::new(),
            loop_config: LoopConfig::default(),
            project_path: project.to_string_lossy().to_string(),
            provider_name: "openai-compatible".to_string(),
            model: "gpt-4o-mini".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "secret".to_string(),
            active_chapter_path: None,
            active_skill: None,
            system_prompt: None,
            suspend_reason: StopReason::WaitingAskuser,
            rounds_executed: 2,
            total_tool_calls: 4,
        },
        4,
    );

    let snapshot_ctx = SnapshotContext {
        project_path: project.to_string_lossy().to_string(),
        session_id: session_id.clone(),
        provider_name: "openai-compatible".to_string(),
        model: "gpt-4o-mini".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        system_prompt: None,
        active_chapter_path: None,
        active_skill: None,
        loop_config: LoopConfig::default(),
    };

    persist_runtime_after_loop(
        &snapshot_ctx,
        &session_id,
        &conversation,
        &StopReason::WaitingAskuser,
        None,
    );

    assert!(session_state::global().has_suspended(&session_id));

    let prepared =
        prepare_resume_turn(&session_id).expect("suspended turn should still be resumable");
    assert_eq!(prepared.turn_id, 3);

    crate::agent_engine::session_state::global().clear_cancel_token(&session_id);
    crate::agent_engine::session_state::global().remove_session(&session_id);
}

#[test]
fn prepare_resume_turn_reuses_suspended_turn_id() {
    let session_id = format!("test_prepare_resume_{}", uuid::Uuid::new_v4());
    let mut conversation = ConversationState::new(session_id.clone());
    conversation.current_turn = 9;

    session_state::global().save_suspended_runtime_state(
        &session_id,
        session_state::SuspendedTurnState {
            conversation_state: conversation,
            pending_tool_call: crate::agent_engine::types::ToolCallInfo {
                llm_call_id: "call_1".to_string(),
                tool_name: "edit".to_string(),
                args: serde_json::json!({"path": "chapter1.md"}),
            },
            pending_call_id: "pending_1".to_string(),
            remaining_tool_calls: Vec::new(),
            completed_messages: Vec::new(),
            loop_config: LoopConfig::default(),
            project_path: "D:/tmp/project".to_string(),
            provider_name: "openai-compatible".to_string(),
            model: "gpt-4o-mini".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "secret".to_string(),
            active_chapter_path: None,
            active_skill: None,
            system_prompt: None,
            suspend_reason: StopReason::WaitingConfirmation,
            rounds_executed: 1,
            total_tool_calls: 1,
        },
        10,
    );

    let prepared = prepare_resume_turn(&session_id).expect("prepare resume turn");
    assert_eq!(prepared.turn_id, 9);
    assert_eq!(session_state::global().peek_next_turn_id(&session_id), 10);

    crate::agent_engine::session_state::global().clear_cancel_token(&session_id);
    crate::agent_engine::session_state::global().remove_session(&session_id);
}

#[test]
fn hydrate_existing_session_on_start_rejects_missing_runtime_for_historical_session() {
    let project = setup_temp_project();
    let session_id = format!("test_missing_runtime_{}", uuid::Uuid::new_v4());

    let stream_path = crate::services::session_stream_path(project.as_path(), &session_id);
    let event = crate::services::AgentSessionEvent {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        event_type: "turn_started".to_string(),
        session_id: session_id.clone(),
        ts: chrono::Utc::now().timestamp_millis(),
        event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
        event_seq: Some(1),
        dedupe_key: None,
        turn: Some(1),
        payload: Some(serde_json::json!({"test": true})),
    };
    crate::services::append_events_jsonl(&stream_path, &[event]).expect("append event");

    let now = chrono::Utc::now().timestamp_millis();
    let meta = crate::services::AgentSessionMeta {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        session_id: session_id.clone(),
        title: Some("historical".to_string()),
        created_at: now,
        updated_at: now,
        last_turn: Some(1),
        last_stop_reason: Some("success".to_string()),
        active_chapter_path: None,
        compaction_count: Some(0),
    };
    crate::application::command_usecases::agent_session_support::save_session_meta(
        project.as_path(),
        meta,
    )
    .expect("save meta");

    crate::services::save_runtime_snapshot_from_input(
            project.as_path(),
            crate::services::RuntimeSnapshotUpsertInput::readonly(
                session_id.clone(),
                crate::application::command_usecases::agent_session::AgentSessionReadonlyReason::RuntimeStateUnavailable
                    .as_str()
                    .to_string(),
                Some(1),
            ),
        )
        .expect("save readonly snapshot");

    let err = hydrate_existing_session_on_start(project.to_string_lossy().as_ref(), &session_id)
        .expect_err("historical session without runtime should fail");

    let details = err.details.expect("error details");
    let code = details
        .get("code")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(code, "E_AGENT_SESSION_RUNTIME_UNAVAILABLE");

    crate::agent_engine::session_state::global().remove_session(&session_id);
}

#[test]
fn hydrate_existing_session_on_start_rejects_resumable_suspended_session() {
    let project = setup_temp_project();
    let session_id = format!("test_suspended_runtime_{}", uuid::Uuid::new_v4());

    let stream_path = crate::services::session_stream_path(project.as_path(), &session_id);
    let event = crate::services::AgentSessionEvent {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        event_type: "turn_state".to_string(),
        session_id: session_id.clone(),
        ts: chrono::Utc::now().timestamp_millis(),
        event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
        event_seq: Some(1),
        dedupe_key: None,
        turn: Some(1),
        payload: Some(serde_json::json!({
            "state": "waiting_confirmation",
            "call_id": "tool_1",
        })),
    };
    crate::services::append_events_jsonl(&stream_path, &[event]).expect("append event");

    let now = chrono::Utc::now().timestamp_millis();
    let meta = crate::services::AgentSessionMeta {
        schema_version: crate::services::AGENT_SESSION_SCHEMA_VERSION,
        session_id: session_id.clone(),
        title: Some("suspended".to_string()),
        created_at: now,
        updated_at: now,
        last_turn: Some(1),
        last_stop_reason: Some("limit".to_string()),
        active_chapter_path: None,
        compaction_count: Some(0),
    };
    crate::application::command_usecases::agent_session_support::save_session_meta(
        project.as_path(),
        meta,
    )
    .expect("save meta");

    let mut conversation = ConversationState::new(session_id.clone());
    conversation
        .messages
        .push(AgentMessage::user("needs confirmation".to_string()));
    conversation.current_turn = 1;

    let suspended = crate::agent_engine::session_state::SuspendedTurnState {
        conversation_state: conversation,
        pending_tool_call: crate::agent_engine::types::ToolCallInfo {
            llm_call_id: "call_1".to_string(),
            tool_name: "edit".to_string(),
            args: serde_json::json!({"path": "chapter1.md"}),
        },
        pending_call_id: "pending_1".to_string(),
        remaining_tool_calls: Vec::new(),
        completed_messages: Vec::new(),
        loop_config: LoopConfig::default(),
        project_path: project.to_string_lossy().to_string(),
        provider_name: "openai-compatible".to_string(),
        model: "gpt-4o-mini".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: "secret".to_string(),
        active_chapter_path: None,
        active_skill: None,
        system_prompt: None,
        suspend_reason: StopReason::WaitingConfirmation,
        rounds_executed: 1,
        total_tool_calls: 1,
    };

    crate::services::save_runtime_snapshot_from_input(
        project.as_path(),
        RuntimeSnapshotUpsertInput::from_suspended(session_id.clone(), suspended, Some(1)),
    )
    .expect("save suspended snapshot");

    let err = hydrate_existing_session_on_start(project.to_string_lossy().as_ref(), &session_id)
        .expect_err("suspended session should require resume");
    let details = err.details.expect("error details");
    let code = details
        .get("code")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(code, "E_AGENT_SESSION_RESUME_NOT_SUPPORTED");

    crate::agent_engine::session_state::global().remove_session(&session_id);
}

#[test]
fn should_persist_explicit_resumed_state_only_for_confirmation() {
    let confirmation = ResumeInput::Confirmation { allowed: true };
    let askuser = ResumeInput::AskUser {
        answers: serde_json::json!([{ "topic": "continue", "value": "yes" }]),
    };

    assert!(should_persist_explicit_resumed_state(&confirmation));
    assert!(!should_persist_explicit_resumed_state(&askuser));
}

#[test]
fn build_confirmation_denied_trace_contains_policy_error_contract() {
    let trace = build_confirmation_denied_trace("edit", "tool_denied_1");

    assert_eq!(trace["schema_version"], 2);
    assert_eq!(trace["stage"], "result");
    assert_eq!(trace["meta"]["tool"], "edit");
    assert_eq!(trace["meta"]["call_id"], "tool_denied_1");
    assert_eq!(trace["result"]["ok"], false);
    assert_eq!(trace["result"]["error"]["code"], "E_TOOL_EXECUTION_DENIED");
    assert_eq!(trace["result"]["error"]["fault_domain"], "policy");
}
