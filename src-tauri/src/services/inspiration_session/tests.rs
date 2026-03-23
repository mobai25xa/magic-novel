use crate::agent_engine::messages::{AgentMessage, ConversationState};
use crate::application::command_usecases::inspiration::{
    ConsensusValue, CreateProjectHandoffDraft, InspirationConsensusState, OpenQuestion,
    OpenQuestionImportance, OpenQuestionStatus,
};
use crate::services::agent_session::SessionRuntimeState;
use crate::test_support::inspiration_env::with_temp_root;

use super::{
    append_session_events, load_runtime_snapshot, load_session_events, load_session_meta,
    save_runtime_snapshot_from_input, save_session_meta, InspirationRuntimeSnapshotUpsertInput,
    InspirationSessionEvent, InspirationSessionMeta, INSPIRATION_SESSION_SCHEMA_VERSION,
};

#[test]
fn inspiration_session_events_roundtrip() {
    with_temp_root(|| {
        let meta = InspirationSessionMeta::new("insp_1".to_string(), 1, Some("idea".to_string()));
        save_session_meta(meta).expect("save meta");

        let events = vec![InspirationSessionEvent {
            schema_version: INSPIRATION_SESSION_SCHEMA_VERSION,
            event_type: "session_start".to_string(),
            session_id: "insp_1".to_string(),
            ts: 1,
            event_id: None,
            event_seq: None,
            dedupe_key: Some("session_start".to_string()),
            turn: None,
            payload: Some(serde_json::json!({ "scope": "inspiration" })),
        }];

        let appended = append_session_events("insp_1", &events).expect("append events");
        assert_eq!(appended.appended_count, 1);
        assert_eq!(appended.last_event_seq, 1);

        let loaded = load_session_events("insp_1").expect("load events");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].event_seq, Some(1));
    });
}

#[test]
fn inspiration_runtime_snapshot_roundtrip() {
    with_temp_root(|| {
        let mut conversation = ConversationState::new("insp_2".to_string());
        conversation.current_turn = 3;
        conversation
            .messages
            .push(AgentMessage::user("seed idea".to_string()));
        let mut consensus = InspirationConsensusState::default();
        consensus.story_core.confirmed_value =
            Some(ConsensusValue::Text("稳定的故事核心".to_string()));
        let open_questions = vec![OpenQuestion {
            question_id: "q_001".to_string(),
            question: "主角真正想要什么？".to_string(),
            importance: OpenQuestionImportance::High,
            status: OpenQuestionStatus::Open,
        }];
        let handoff = CreateProjectHandoffDraft {
            name: "雨夜禁书".to_string(),
            description: "一名落魄抄经师被卷入禁书争夺。".to_string(),
            project_type: vec!["奇幻".to_string()],
            tone: vec!["克制".to_string()],
            audience: "奇幻读者".to_string(),
            protagonist_seed: Some("抄经师".to_string()),
            counterpart_seed: Some("审校官".to_string()),
            world_seed: Some("禁书改写现实".to_string()),
            ending_direction: Some("赢下真相但失去记忆".to_string()),
        };

        let mut input = InspirationRuntimeSnapshotUpsertInput::from_conversation(
            "insp_2".to_string(),
            SessionRuntimeState::Completed,
            conversation,
            Some(3),
            Some("openai-compatible".to_string()),
            Some("gpt-4o-mini".to_string()),
            Some("https://example.com/v1".to_string()),
            Some("system".to_string()),
            None,
        );
        input.consensus = Some(consensus.clone());
        input.open_questions = Some(open_questions.clone());
        input.final_create_handoff_draft = Some(handoff.clone());

        save_runtime_snapshot_from_input(input).expect("save runtime snapshot");

        let snapshot = load_runtime_snapshot("insp_2")
            .expect("load runtime snapshot")
            .expect("snapshot");
        assert_eq!(snapshot.runtime_state, SessionRuntimeState::Completed);
        assert_eq!(snapshot.last_turn, Some(3));
        assert_eq!(snapshot.next_turn_id, Some(4));
        assert_eq!(snapshot.consensus, Some(consensus));
        assert_eq!(snapshot.open_questions, Some(open_questions));
        assert_eq!(snapshot.final_create_handoff_draft, Some(handoff));
    });
}

#[test]
fn inspiration_meta_is_saved_and_loaded() {
    with_temp_root(|| {
        let meta = InspirationSessionMeta::new(
            "insp_3".to_string(),
            42,
            Some("My Inspiration".to_string()),
        );
        save_session_meta(meta.clone()).expect("save meta");

        let loaded = load_session_meta("insp_3")
            .expect("load meta")
            .expect("meta");
        assert_eq!(loaded.session_id, meta.session_id);
        assert_eq!(loaded.title, meta.title);
    });
}
