use crate::agent_engine::exposure_policy::{
    CapabilityPolicy, CapabilityPreset, ExposureContext, SessionSource,
};
use crate::agent_engine::messages::{AgentMessage, ContentBlock, ConversationState, Role};
use crate::agent_engine::types::{AgentMode, ApprovalMode, ClarificationMode};
use crate::services::agent_session::SessionRuntimeState;
use crate::services::inspiration_session::{
    append_session_events, load_runtime_snapshot, load_session_meta, remove_session_meta,
    save_runtime_snapshot_from_input, session_event_types, InspirationRuntimeSnapshotUpsertInput,
    InspirationSessionEvent, INSPIRATION_SESSION_SCHEMA_VERSION,
};
use crate::test_support::inspiration_env::with_temp_root;

use crate::agent_tools::definition::ToolSchemaContext;
use crate::agent_tools::registry::{build_openai_tool_schema_report_for_exposure, get_schema};

use super::{
    apply_consensus_patch, apply_open_questions_patch, build_create_handoff,
    create_inspiration_session, ensure_inspiration_session_exists, generate_metadata_variants,
    load_inspiration_session_snapshot, save_inspiration_session_state, ApplyConsensusPatchInput,
    ApplyConsensusPatchOutput, ApplyOpenQuestionsPatchInput, ApplyOpenQuestionsPatchOutput,
    ConsensusFieldId, ConsensusPatchOperation, ConsensusValue, CreateProjectHandoffDraft,
    GenerateMetadataVariantsInput, InspirationConsensusState, MetadataVariantId, OpenQuestion,
    OpenQuestionImportance, OpenQuestionStatus, OpenQuestionsPatchOperation,
    HYDRATION_STATUS_SNAPSHOT_LOADED,
};

#[test]
fn locked_field_cannot_be_overwritten() {
    let mut state = InspirationConsensusState::default();
    state.premise.locked = true;
    state.premise.confirmed_value = Some(ConsensusValue::Text("旧 premise".to_string()));

    let err = apply_consensus_patch(ApplyConsensusPatchInput {
        state,
        field_id: ConsensusFieldId::Premise,
        operation: ConsensusPatchOperation::SetText,
        text_value: Some("新 premise".to_string()),
        items: Vec::new(),
        source_turn_id: Some(3),
    })
    .expect_err("locked field should reject patch");

    assert!(err.message.contains("locked"));
}

#[test]
fn create_and_load_inspiration_session_without_project_path() {
    with_temp_root(|| {
        let (session_id, created_at) =
            create_inspiration_session(Some("灵感策划".to_string())).expect("create session");

        assert!(session_id.starts_with("insp_"));
        assert!(created_at > 0);
        ensure_inspiration_session_exists(&session_id).expect("session should exist");

        let loaded = load_inspiration_session_snapshot(&session_id).expect("load snapshot");
        assert_eq!(loaded.meta.session_id, session_id);
        assert_eq!(loaded.meta.title.as_deref(), Some("灵感策划"));
        assert_eq!(loaded.runtime_state, SessionRuntimeState::Ready);
        assert_eq!(loaded.hydration_status, HYDRATION_STATUS_SNAPSHOT_LOADED);
        assert_eq!(loaded.last_turn, Some(0));
        assert_eq!(loaded.next_turn_id, Some(1));
        assert!(loaded.conversation.messages.is_empty());
        assert_eq!(loaded.events.len(), 1);
        assert_eq!(loaded.events[0].event_type, "session_start");
    });
}

#[test]
fn inspiration_session_reload_keeps_independent_turn_counter() {
    with_temp_root(|| {
        let (session_id, _) = create_inspiration_session(None).expect("create session");

        let first = load_inspiration_session_snapshot(&session_id).expect("first load");
        let second = load_inspiration_session_snapshot(&session_id).expect("second load");

        assert_eq!(first.meta.session_id, second.meta.session_id);
        assert_eq!(first.runtime_state, SessionRuntimeState::Ready);
        assert_eq!(second.runtime_state, SessionRuntimeState::Ready);
        assert_eq!(first.last_turn, Some(0));
        assert_eq!(second.last_turn, Some(0));
        assert_eq!(first.next_turn_id, Some(1));
        assert_eq!(second.next_turn_id, Some(1));
    });
}

#[test]
fn missing_meta_is_recovered_from_persisted_session_files() {
    with_temp_root(|| {
        let (session_id, _) =
            create_inspiration_session(Some("会被修复的会话".to_string())).expect("create session");

        remove_session_meta(&session_id).expect("remove only meta index entry");
        let repaired =
            load_inspiration_session_snapshot(&session_id).expect("load repaired snapshot");

        assert_eq!(repaired.meta.session_id, session_id);
        assert!(repaired.meta.title.is_none());
        assert_eq!(repaired.events.len(), 1);
        assert_eq!(repaired.events[0].event_type, "session_start");
        ensure_inspiration_session_exists(&session_id).expect("repaired session should exist");

        let persisted_meta = load_session_meta(&session_id)
            .expect("reload repaired meta")
            .expect("repaired meta should be persisted");
        assert_eq!(persisted_meta.session_id, repaired.meta.session_id);
        assert_eq!(persisted_meta.last_turn, repaired.meta.last_turn);
    });
}

#[test]
fn load_snapshot_derives_domain_state_from_conversation_when_snapshot_fields_are_missing() {
    with_temp_root(|| {
        let (session_id, _) = create_inspiration_session(None).expect("create session");

        let mut conversation = ConversationState::new(session_id.clone());
        conversation.current_turn = 2;
        conversation
            .messages
            .push(AgentMessage::user("先聊一个灵感".to_string()));

        let consensus_output = ApplyConsensusPatchOutput {
            field_id: ConsensusFieldId::Premise,
            operation: ConsensusPatchOperation::SetText,
            updated_field: {
                let mut field = InspirationConsensusState::default().premise;
                field.draft_value = Some(ConsensusValue::Text("一句话 premise".to_string()));
                field.last_source_turn_id = Some(2);
                field.updated_at = 123;
                field
            },
            state: {
                let mut state = InspirationConsensusState::default();
                state.premise.draft_value =
                    Some(ConsensusValue::Text("一句话 premise".to_string()));
                state.premise.last_source_turn_id = Some(2);
                state.premise.updated_at = 123;
                state
            },
        };
        conversation.messages.push(AgentMessage::tool_result(
            "call_consensus".to_string(),
            Some("inspiration_consensus_patch".to_string()),
            serde_json::to_string(&consensus_output).expect("serialize consensus output"),
            false,
        ));

        let question = OpenQuestion {
            question_id: "q_001".to_string(),
            question: "主角真正想要什么？".to_string(),
            importance: OpenQuestionImportance::High,
            status: OpenQuestionStatus::Open,
        };
        let open_questions_output = ApplyOpenQuestionsPatchOutput {
            operation: OpenQuestionsPatchOperation::Add,
            updated_question: question.clone(),
            questions: vec![question.clone()],
        };
        conversation.messages.push(AgentMessage::tool_result(
            "call_open_question".to_string(),
            Some("inspiration_open_questions_patch".to_string()),
            serde_json::to_string(&open_questions_output).expect("serialize open questions output"),
            false,
        ));

        save_runtime_snapshot_from_input(InspirationRuntimeSnapshotUpsertInput::from_conversation(
            session_id.clone(),
            SessionRuntimeState::Completed,
            conversation,
            Some(2),
            None,
            None,
            None,
            None,
            None,
        ))
        .expect("save runtime snapshot");

        let loaded = load_inspiration_session_snapshot(&session_id).expect("load snapshot");
        assert_eq!(
            loaded.consensus.premise.draft_value,
            Some(ConsensusValue::Text("一句话 premise".to_string()))
        );
        assert_eq!(loaded.open_questions, vec![question]);
        assert!(loaded.final_create_handoff_draft.is_none());
    });
}

#[test]
fn load_snapshot_merges_newer_event_messages_ahead_of_stale_runtime_snapshot() {
    with_temp_root(|| {
        let (session_id, _) = create_inspiration_session(None).expect("create session");

        let mut conversation = ConversationState::new(session_id.clone());
        conversation.current_turn = 1;
        conversation
            .messages
            .push(AgentMessage::user("先给我一个故事方向".to_string()));

        let snapshot = save_runtime_snapshot_from_input(
            InspirationRuntimeSnapshotUpsertInput::from_conversation(
                session_id.clone(),
                SessionRuntimeState::Running,
                conversation,
                Some(1),
                None,
                None,
                None,
                None,
                None,
            ),
        )
        .expect("save stale running snapshot");

        append_session_events(
            &session_id,
            &[
                InspirationSessionEvent {
                    schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
                    event_type: session_event_types::MESSAGE.to_string(),
                    session_id: session_id.clone(),
                    ts: snapshot.updated_at + 10,
                    event_id: Some("evt_assistant_message".to_string()),
                    event_seq: None,
                    dedupe_key: None,
                    turn: Some(1),
                    payload: Some(serde_json::json!({
                        "role": "assistant",
                        "content": "这是已经写入事件流的最终回答。",
                        "message_id": "msg_assistant_final",
                    })),
                },
                InspirationSessionEvent {
                    schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
                    event_type: session_event_types::TURN_COMPLETED.to_string(),
                    session_id: session_id.clone(),
                    ts: snapshot.updated_at + 11,
                    event_id: Some("evt_turn_completed".to_string()),
                    event_seq: None,
                    dedupe_key: None,
                    turn: Some(1),
                    payload: Some(serde_json::json!({
                        "stop_reason": "success",
                    })),
                },
            ],
        )
        .expect("append fresher persisted events");

        let loaded = load_inspiration_session_snapshot(&session_id).expect("load merged snapshot");

        assert_eq!(loaded.conversation.messages.len(), 2);
        assert_eq!(
            loaded.conversation.messages[0].text_content(),
            "先给我一个故事方向"
        );
        assert_eq!(
            loaded.conversation.messages[1].text_content(),
            "这是已经写入事件流的最终回答。"
        );
        assert_eq!(loaded.runtime_state, SessionRuntimeState::Completed);
        assert_eq!(loaded.last_turn, Some(1));
        assert_eq!(loaded.next_turn_id, Some(2));
    });
}

#[test]
fn load_snapshot_dedupes_runtime_and_event_user_message_identity_mismatch() {
    with_temp_root(|| {
        let (session_id, _) = create_inspiration_session(None).expect("create session");

        let user_text = "hi".to_string();
        let conversation = ConversationState {
            session_id: session_id.clone(),
            messages: vec![AgentMessage {
                id: "msg_user_runtime".to_string(),
                role: Role::User,
                blocks: vec![ContentBlock::Text {
                    text: user_text.clone(),
                }],
                ts: 100,
            }],
            current_turn: 1,
            total_tool_calls: 0,
            last_compaction: None,
            last_usage: None,
        };

        save_runtime_snapshot_from_input(InspirationRuntimeSnapshotUpsertInput::from_conversation(
            session_id.clone(),
            SessionRuntimeState::Running,
            conversation,
            Some(1),
            None,
            None,
            None,
            None,
            None,
        ))
        .expect("save runtime snapshot");

        append_session_events(
            &session_id,
            &[
                InspirationSessionEvent {
                    schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
                    event_type: session_event_types::MESSAGE.to_string(),
                    session_id: session_id.clone(),
                    ts: 101,
                    event_id: Some("evt_user_message".to_string()),
                    event_seq: None,
                    dedupe_key: None,
                    turn: Some(1),
                    payload: Some(serde_json::json!({
                        "role": "user",
                        "content": user_text,
                        "message_id": "msg_user_event",
                    })),
                },
                InspirationSessionEvent {
                    schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
                    event_type: session_event_types::MESSAGE.to_string(),
                    session_id: session_id.clone(),
                    ts: 102,
                    event_id: Some("evt_assistant_message".to_string()),
                    event_seq: None,
                    dedupe_key: None,
                    turn: Some(1),
                    payload: Some(serde_json::json!({
                        "role": "assistant",
                        "content": "你好，我可以帮你一起完善故事构思。",
                        "message_id": "msg_assistant_final",
                    })),
                },
            ],
        )
        .expect("append runtime and event messages");

        let loaded = load_inspiration_session_snapshot(&session_id).expect("load snapshot");

        assert_eq!(loaded.conversation.messages.len(), 2);
        assert_eq!(loaded.conversation.messages[0].text_content(), "hi");
        assert_eq!(
            loaded.conversation.messages[1].text_content(),
            "你好，我可以帮你一起完善故事构思。"
        );
    });
}

#[test]
fn save_inspiration_state_persists_consensus_open_questions_and_handoff() {
    with_temp_root(|| {
        let (session_id, _) = create_inspiration_session(None).expect("create session");
        let mut consensus = InspirationConsensusState::default();
        consensus.story_core.confirmed_value =
            Some(ConsensusValue::Text("稳定的故事核心".to_string()));
        consensus.story_core.locked = true;
        consensus.premise.draft_value = Some(ConsensusValue::Text(
            "主角在雨夜收到一封禁书邀请".to_string(),
        ));

        let open_questions = vec![OpenQuestion {
            question_id: "q_002".to_string(),
            question: "这本书更偏悬疑还是偏情感？".to_string(),
            importance: OpenQuestionImportance::Medium,
            status: OpenQuestionStatus::Open,
        }];
        let handoff = CreateProjectHandoffDraft {
            name: "雨夜禁书".to_string(),
            description: "一名落魄抄经师被卷入禁书与秩序的争夺。".to_string(),
            project_type: vec!["奇幻".to_string(), "悬疑".to_string()],
            tone: vec!["克制".to_string()],
            audience: "设定驱动奇幻读者".to_string(),
            protagonist_seed: Some("落魄抄经师".to_string()),
            counterpart_seed: Some("追查禁书的审校官".to_string()),
            world_seed: Some("禁书可改写现实但要吞噬记忆".to_string()),
            ending_direction: Some("赢下真相但付出记忆代价".to_string()),
        };

        let saved = save_inspiration_session_state(
            &session_id,
            consensus.clone(),
            open_questions.clone(),
            Some(handoff.clone()),
        )
        .expect("save state");

        assert_eq!(saved.consensus, consensus);
        assert_eq!(saved.open_questions, open_questions);
        assert_eq!(saved.final_create_handoff_draft, Some(handoff.clone()));

        let snapshot = load_runtime_snapshot(&session_id)
            .expect("load runtime snapshot")
            .expect("snapshot should exist");
        assert_eq!(snapshot.consensus, Some(consensus));
        assert_eq!(snapshot.open_questions, Some(open_questions));
        assert_eq!(snapshot.final_create_handoff_draft, Some(handoff));
    });
}

#[test]
fn open_question_patch_supports_add_resolve_and_dismiss() {
    let added = apply_open_questions_patch(ApplyOpenQuestionsPatchInput {
        questions: Vec::new(),
        operation: OpenQuestionsPatchOperation::Add,
        question_id: None,
        question: Some("主角真正想要什么？".to_string()),
        importance: Some(OpenQuestionImportance::High),
    })
    .expect("add question");

    assert_eq!(added.questions.len(), 1);
    assert_eq!(added.updated_question.question_id, "q_001");

    let resolved = apply_open_questions_patch(ApplyOpenQuestionsPatchInput {
        questions: added.questions.clone(),
        operation: OpenQuestionsPatchOperation::Resolve,
        question_id: Some("q_001".to_string()),
        question: None,
        importance: None,
    })
    .expect("resolve question");

    assert_eq!(
        resolved.updated_question.status,
        OpenQuestionStatus::Resolved
    );

    let dismissed = apply_open_questions_patch(ApplyOpenQuestionsPatchInput {
        questions: resolved.questions,
        operation: OpenQuestionsPatchOperation::Dismiss,
        question_id: Some("q_001".to_string()),
        question: None,
        importance: None,
    })
    .expect("dismiss question");

    assert_eq!(
        dismissed.updated_question.status,
        OpenQuestionStatus::Dismissed
    );
}

#[tokio::test]
async fn variant_generation_returns_fixed_ids_and_shared_story_core() {
    let mut consensus = InspirationConsensusState::default();
    consensus.story_core.draft_value = Some(ConsensusValue::Text(
        "一个被放逐的抄经师必须在禁书与秩序之间做选择".to_string(),
    ));
    consensus.premise.draft_value = Some(ConsensusValue::Text(
        "被放逐的抄经师发现禁书能改写现实，却会吞噬记忆。".to_string(),
    ));
    consensus.genre_tone.draft_value = Some(ConsensusValue::List(vec![
        "奇幻".to_string(),
        "悬疑".to_string(),
        "克制".to_string(),
    ]));
    consensus.protagonist.draft_value = Some(ConsensusValue::Text(
        "主角是谨慎、自律、擅长辨伪的抄经师，但内心一直想证明自己不是废人。".to_string(),
    ));
    consensus.worldview.draft_value = Some(ConsensusValue::Text(
        "帝国以抄写院垄断知识，禁书会以代价换来现实改写。".to_string(),
    ));
    consensus.core_conflict.draft_value = Some(ConsensusValue::Text(
        "主角必须决定是交出禁书保全秩序，还是借禁书撕开真相。".to_string(),
    ));
    consensus.selling_points.draft_value = Some(ConsensusValue::List(vec![
        "禁书机制".to_string(),
        "记忆代价".to_string(),
    ]));
    consensus.audience.draft_value = Some(ConsensusValue::Text("偏女频".to_string()));
    consensus.ending_direction.draft_value = Some(ConsensusValue::Text(
        "结局应让主角赢下真相，但失去一部分最重要的记忆。".to_string(),
    ));

    let output = generate_metadata_variants(GenerateMetadataVariantsInput { consensus })
        .await
        .expect("variants should generate");

    let ids = output
        .variants
        .iter()
        .map(|variant| variant.variant_id)
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            MetadataVariantId::Balanced,
            MetadataVariantId::Hook,
            MetadataVariantId::Setting,
        ]
    );
    assert_eq!(
        output.shared_story_core,
        "一个被放逐的抄经师必须在禁书与秩序之间做选择"
    );

    let handoff = build_create_handoff(&output.variants[0]);
    assert!(!handoff.name.is_empty());
    assert!(!handoff.description.is_empty());
    assert!(!handoff.project_type.is_empty());
}

#[tokio::test]
async fn variant_generation_rejects_missing_required_consensus() {
    let err = generate_metadata_variants(GenerateMetadataVariantsInput {
        consensus: InspirationConsensusState::default(),
    })
    .await
    .expect_err("missing required fields should fail");

    assert!(err.message.contains("story_core"));
    assert!(err.message.contains("premise"));
    assert!(err.message.contains("genre_tone"));
}

#[test]
fn inspiration_tools_are_hidden_by_default_but_available_by_forced_policy() {
    let context = ToolSchemaContext::default();
    assert!(
        get_schema("inspiration_consensus_patch", &context).is_some(),
        "schema should exist"
    );

    let mut policy = CapabilityPolicy::new(CapabilityPreset::MainPlanning);
    policy.forced_tools = vec![
        "inspiration_consensus_patch".to_string(),
        "inspiration_open_questions_patch".to_string(),
    ];
    let exposure = ExposureContext::new(
        AgentMode::Planning,
        ApprovalMode::Auto,
        ClarificationMode::Interactive,
        SessionSource::UserInteractive,
        0,
        false,
        None,
        Some("inspiration_session".to_string()),
        policy,
    );

    let report = build_openai_tool_schema_report_for_exposure(&exposure, &context);

    assert_eq!(
        report.exposed_tools,
        vec![
            "inspiration_consensus_patch".to_string(),
            "inspiration_open_questions_patch".to_string(),
        ]
    );
}
