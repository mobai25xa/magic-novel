#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use serde_json::json;

    use crate::agent_engine::messages::{AgentMessage, ConversationState};
    use crate::application::command_usecases::agent_session::{
        hydrate_runtime_state, AgentSessionHydrateOutput,
    };
    use crate::application::command_usecases::agent_session_support::delete_session;
    use crate::application::command_usecases::agent_session_support::{
        load_session_meta as load_session_meta_from_support, save_session_meta,
    };
    use crate::services::agent_session::types::session_event_types;
    use crate::services::agent_session::{
        append_events_jsonl, append_session_events, find_meta, load_index, read_events_jsonl,
        recover_stream_file, runtime_snapshot_path, save_index, save_runtime_snapshot_from_input,
        session_index_path, session_stream_path, AgentSessionEvent, AgentSessionMeta,
        RuntimeSnapshotUpsertInput, SessionRuntimeState, AGENT_SESSION_SCHEMA_VERSION,
    };

    fn setup_temp_project() -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("magic_session_test_{}", uuid::Uuid::new_v4()));
        let sessions_dir = base.join("magic_novel").join("ai").join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        base
    }

    fn make_event(event_type: &str, session_id: &str, turn: i64) -> AgentSessionEvent {
        AgentSessionEvent {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            event_type: event_type.to_string(),
            session_id: session_id.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
            event_seq: Some(turn),
            dedupe_key: None,
            turn: Some(turn),
            payload: Some(json!({"test": true})),
        }
    }

    fn ensure_meta(project: &PathBuf, session_id: &str) {
        let now = chrono::Utc::now().timestamp_millis();
        let meta = AgentSessionMeta {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            title: Some("test session".to_string()),
            created_at: now,
            updated_at: now,
            last_turn: Some(1),
            last_stop_reason: Some("success".to_string()),
            active_chapter_path: None,
            compaction_count: Some(0),
        };
        save_session_meta(project.as_path(), meta).unwrap();
    }

    fn hydrate(project: &PathBuf, session_id: &str) -> AgentSessionHydrateOutput {
        hydrate_runtime_state(
            project.as_path(),
            project.to_string_lossy().as_ref(),
            session_id,
        )
        .unwrap()
    }

    // ── Stream parser round-trip ────────────────────────────────

    #[test]
    fn test_stream_round_trip() {
        let project = setup_temp_project();
        let session_id = "test_rt";
        let stream_path = session_stream_path(&project, session_id);

        let events = vec![
            make_event(session_event_types::TURN_STARTED, session_id, 1),
            make_event(session_event_types::MESSAGE, session_id, 1),
            make_event(session_event_types::TURN_COMPLETED, session_id, 1),
        ];

        append_events_jsonl(&stream_path, &events).unwrap();

        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].event_type, session_event_types::TURN_STARTED);
        assert_eq!(loaded[1].event_type, session_event_types::MESSAGE);
        assert_eq!(loaded[2].event_type, session_event_types::TURN_COMPLETED);

        for (original, loaded) in events.iter().zip(loaded.iter()) {
            assert_eq!(original.session_id, loaded.session_id);
            assert_eq!(original.event_type, loaded.event_type);
            assert_eq!(original.event_id, loaded.event_id);
        }
    }

    // ── Partial line skip ───────────────────────────────────────

    #[test]
    fn test_partial_line_skip() {
        let project = setup_temp_project();
        let session_id = "test_partial";
        let stream_path = session_stream_path(&project, session_id);

        let events = vec![
            make_event(session_event_types::TURN_STARTED, session_id, 1),
            make_event(session_event_types::MESSAGE, session_id, 1),
        ];
        append_events_jsonl(&stream_path, &events).unwrap();

        // Manually append a truncated (partial) line
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&stream_path)
            .unwrap();
        file.write_all(b"{\"broken\":\"trun\n").unwrap();

        // read_events_jsonl should skip the partial line that doesn't end with '}'
        // Note: the current implementation skips lines that don't end with '}'
        // but our partial line `{"broken":"trun` does not end with '}'
        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    // ── Recovery ────────────────────────────────────────────────

    #[test]
    fn test_recovery_truncates_bad_line() {
        let project = setup_temp_project();
        let session_id = "test_recover";
        let stream_path = session_stream_path(&project, session_id);

        let events = vec![
            make_event(session_event_types::TURN_STARTED, session_id, 1),
            make_event(session_event_types::MESSAGE, session_id, 1),
            make_event(session_event_types::TURN_COMPLETED, session_id, 1),
        ];
        append_events_jsonl(&stream_path, &events).unwrap();

        // Append a corrupt line
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&stream_path)
            .unwrap();
        file.write_all(b"{\"broken\":\n").unwrap();

        // Recover should truncate
        let (truncated, reason) = recover_stream_file(&stream_path).unwrap();
        assert!(truncated > 0, "should have truncated bytes");
        assert!(reason.is_some(), "should have a reason");

        // After recovery, should be able to read the 3 valid events
        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert_eq!(loaded.len(), 3);

        // Should be able to append more events after recovery
        let new_event = make_event(session_event_types::TURN_STARTED, session_id, 2);
        append_events_jsonl(&stream_path, &[new_event]).unwrap();

        let loaded_after = read_events_jsonl(&stream_path).unwrap();
        assert_eq!(loaded_after.len(), 4);
    }

    // ── validate_v1 ─────────────────────────────────────────────

    #[test]
    fn test_validate_v1_valid() {
        let event = make_event(session_event_types::TURN_STARTED, "session_1", 1);
        assert!(event.validate_v1());
    }

    #[test]
    fn test_validate_v1_wrong_version() {
        let mut event = make_event(session_event_types::TURN_STARTED, "session_1", 1);
        event.schema_version = 999;
        assert!(!event.validate_v1());
    }

    #[test]
    fn test_validate_v1_empty_type() {
        let mut event = make_event(session_event_types::TURN_STARTED, "session_1", 1);
        event.event_type = "  ".to_string();
        assert!(!event.validate_v1());
    }

    #[test]
    fn test_validate_v1_empty_session_id() {
        let mut event = make_event(session_event_types::TURN_STARTED, "session_1", 1);
        event.session_id = "".to_string();
        assert!(!event.validate_v1());
    }

    #[test]
    fn test_validate_v1_zero_ts() {
        let mut event = make_event(session_event_types::TURN_STARTED, "session_1", 1);
        event.ts = 0;
        assert!(!event.validate_v1());
    }

    // ── New event types serialize/deserialize ───────────────────

    #[test]
    fn test_turn_state_event_round_trip() {
        let project = setup_temp_project();
        let session_id = "test_turn_state";
        let stream_path = session_stream_path(&project, session_id);

        let event = AgentSessionEvent {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            event_type: session_event_types::TURN_STATE.to_string(),
            session_id: session_id.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some("evt_ts_1".to_string()),
            event_seq: Some(1),
            dedupe_key: None,
            turn: Some(1),
            payload: Some(json!({
                "state": "waiting_confirmation",
                "call_id": "tool_42",
                "tool_name": "edit",
            })),
        };

        append_events_jsonl(&stream_path, &[event.clone()]).unwrap();
        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].event_type, session_event_types::TURN_STATE);

        let payload = loaded[0].payload.as_ref().unwrap();
        assert_eq!(payload["state"], "waiting_confirmation");
        assert_eq!(payload["call_id"], "tool_42");
    }

    #[test]
    fn test_compaction_events_round_trip() {
        let project = setup_temp_project();
        let session_id = "test_compaction";
        let stream_path = session_stream_path(&project, session_id);

        let events = vec![
            AgentSessionEvent {
                schema_version: AGENT_SESSION_SCHEMA_VERSION,
                event_type: session_event_types::COMPACTION_STARTED.to_string(),
                session_id: session_id.to_string(),
                ts: chrono::Utc::now().timestamp_millis(),
                event_id: Some("evt_cs_1".to_string()),
                event_seq: Some(1),
                dedupe_key: None,
                turn: Some(1),
                payload: Some(json!({"reason": "threshold"})),
            },
            AgentSessionEvent {
                schema_version: AGENT_SESSION_SCHEMA_VERSION,
                event_type: session_event_types::COMPACTION_SUMMARY.to_string(),
                session_id: session_id.to_string(),
                ts: chrono::Utc::now().timestamp_millis(),
                event_id: Some("evt_cs_2".to_string()),
                event_seq: Some(2),
                dedupe_key: None,
                turn: Some(1),
                payload: Some(json!({
                    "summary_text": "Conversation about novel editing",
                    "removed_count": 10,
                    "keep_recent_count": 6,
                })),
            },
            AgentSessionEvent {
                schema_version: AGENT_SESSION_SCHEMA_VERSION,
                event_type: session_event_types::COMPACTION_FINISHED.to_string(),
                session_id: session_id.to_string(),
                ts: chrono::Utc::now().timestamp_millis(),
                event_id: Some("evt_cf_1".to_string()),
                event_seq: Some(3),
                dedupe_key: None,
                turn: Some(1),
                payload: Some(json!({"meta": {"removed_count": 10}})),
            },
            AgentSessionEvent {
                schema_version: AGENT_SESSION_SCHEMA_VERSION,
                event_type: session_event_types::COMPACTION_FALLBACK.to_string(),
                session_id: session_id.to_string(),
                ts: chrono::Utc::now().timestamp_millis(),
                event_id: Some("evt_cf_2".to_string()),
                event_seq: Some(4),
                dedupe_key: None,
                turn: Some(1),
                payload: Some(json!({"reason": "missing_credentials"})),
            },
        ];

        append_events_jsonl(&stream_path, &events).unwrap();
        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert_eq!(loaded.len(), 4);
        assert_eq!(
            loaded[0].event_type,
            session_event_types::COMPACTION_STARTED
        );
        assert_eq!(
            loaded[1].event_type,
            session_event_types::COMPACTION_SUMMARY
        );
        assert_eq!(
            loaded[2].event_type,
            session_event_types::COMPACTION_FINISHED
        );
        assert_eq!(
            loaded[3].event_type,
            session_event_types::COMPACTION_FALLBACK
        );
    }

    // ── Empty stream ────────────────────────────────────────────

    #[test]
    fn test_read_nonexistent_stream() {
        let project = setup_temp_project();
        let stream_path = session_stream_path(&project, "nonexistent");
        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_read_empty_stream() {
        let project = setup_temp_project();
        let session_id = "test_empty";
        let stream_path = session_stream_path(&project, session_id);
        fs::write(&stream_path, "").unwrap();
        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_read_blank_lines() {
        let project = setup_temp_project();
        let session_id = "test_blanks";
        let stream_path = session_stream_path(&project, session_id);
        fs::write(&stream_path, "\n\n  \n").unwrap();
        let loaded = read_events_jsonl(&stream_path).unwrap();
        assert!(loaded.is_empty());
    }

    // ── Recovery on clean file ──────────────────────────────────

    #[test]
    fn test_recovery_on_clean_file() {
        let project = setup_temp_project();
        let session_id = "test_clean";
        let stream_path = session_stream_path(&project, session_id);

        let events = vec![make_event(session_event_types::TURN_STARTED, session_id, 1)];
        append_events_jsonl(&stream_path, &events).unwrap();

        let (truncated, reason) = recover_stream_file(&stream_path).unwrap();
        assert_eq!(truncated, 0);
        assert!(reason.is_none());
    }

    #[test]
    fn test_recovery_nonexistent() {
        let project = setup_temp_project();
        let stream_path = session_stream_path(&project, "nonexistent");
        let (truncated, reason) = recover_stream_file(&stream_path).unwrap();
        assert_eq!(truncated, 0);
        assert!(reason.is_none());
    }

    #[test]
    fn test_read_events_jsonl_rejects_legacy_schema() {
        let project = setup_temp_project();
        let session_id = "test_migrate";
        let stream_path = session_stream_path(&project, session_id);

        let mut legacy = make_event(session_event_types::TURN_STARTED, session_id, 1);
        legacy.schema_version = 0;
        append_events_jsonl(&stream_path, &[legacy]).unwrap();

        let err = read_events_jsonl(&stream_path).unwrap_err();
        assert!(matches!(
            err.code,
            crate::models::ErrorCode::SchemaValidationError
        ));
        assert_eq!(
            err.details
                .as_ref()
                .and_then(|details| details.get("code"))
                .and_then(|value| value.as_str()),
            Some("E_AGENT_SESSION_LOAD_UNSUPPORTED_SCHEMA")
        );
    }

    #[test]
    fn test_hydrate_event_rebuilt_from_history() {
        let project = setup_temp_project();
        let session_id = "test_hydrate_event_rebuilt";
        let stream_path = session_stream_path(&project, session_id);

        let user_event = AgentSessionEvent {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            event_type: session_event_types::MESSAGE.to_string(),
            session_id: session_id.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
            event_seq: Some(1),
            dedupe_key: None,
            turn: Some(1),
            payload: Some(json!({
                "role": "user",
                "content": "hello from persisted history",
                "message_id": "msg_user_1",
            })),
        };

        append_events_jsonl(&stream_path, &[user_event]).unwrap();
        ensure_meta(&project, session_id);

        let output = hydrate(&project, session_id);
        assert_eq!(output.hydration_status, "event_rebuilt");
        assert_eq!(output.runtime_state, "ready");
        assert!(output.can_continue);
        assert!(!output.can_resume);
        assert_eq!(output.next_turn_id, Some(2));
    }

    #[test]
    fn test_hydrate_snapshot_loaded_for_completed_session() {
        let project = setup_temp_project();
        let session_id = "test_hydrate_snapshot_loaded";
        let stream_path = session_stream_path(&project, session_id);

        let start_event = make_event(session_event_types::TURN_STARTED, session_id, 1);
        append_events_jsonl(&stream_path, &[start_event]).unwrap();
        ensure_meta(&project, session_id);

        let mut conversation = ConversationState::new(session_id.to_string());
        conversation
            .messages
            .push(AgentMessage::user("persisted message".to_string()));
        save_runtime_snapshot_from_input(
            project.as_path(),
            RuntimeSnapshotUpsertInput::from_conversation(
                session_id.to_string(),
                SessionRuntimeState::Completed,
                conversation,
                Some(1),
                Some("openai-compatible".to_string()),
                Some("gpt-4o-mini".to_string()),
                Some("https://api.openai.com/v1".to_string()),
                None,
                None,
                None,
            )
            .with_active_skill(Some("story-architect".to_string())),
        )
        .unwrap();

        let output = hydrate(&project, session_id);
        assert_eq!(output.hydration_status, "snapshot_loaded");
        assert_eq!(output.runtime_state, "completed");
        assert!(output.can_continue);
        assert!(!output.can_resume);
        assert_eq!(output.next_turn_id, Some(2));
        assert_eq!(output.active_skill.as_deref(), Some("story-architect"));
    }

    #[test]
    fn test_hydrate_readonly_fallback_for_suspended_without_snapshot() {
        let project = setup_temp_project();
        let session_id = "test_hydrate_readonly_suspended";
        let stream_path = session_stream_path(&project, session_id);

        let suspended_event = AgentSessionEvent {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            event_type: session_event_types::TURN_STATE.to_string(),
            session_id: session_id.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
            event_seq: Some(1),
            dedupe_key: None,
            turn: Some(1),
            payload: Some(json!({
                "state": "waiting_confirmation",
                "call_id": "tool_1",
            })),
        };

        append_events_jsonl(&stream_path, &[suspended_event]).unwrap();
        ensure_meta(&project, session_id);

        let output = hydrate(&project, session_id);
        assert_eq!(output.hydration_status, "readonly_fallback");
        assert_eq!(output.runtime_state, "degraded");
        assert!(!output.can_continue);
        assert!(!output.can_resume);
        assert_eq!(
            output.readonly_reason.as_deref(),
            Some("historical_suspended_session_without_runtime_snapshot")
        );
        assert_eq!(output.next_turn_id, Some(1));

        let snapshot_path = runtime_snapshot_path(project.as_path(), session_id);
        assert!(snapshot_path.exists());
    }

    #[test]
    fn test_hydrate_memory_hit_from_in_memory_conversation() {
        let project = setup_temp_project();
        let session_id = format!("test_hydrate_memory_hit_{}", uuid::Uuid::new_v4());
        let stream_path = session_stream_path(&project, &session_id);

        append_events_jsonl(
            &stream_path,
            &[make_event(
                session_event_types::TURN_STARTED,
                &session_id,
                1,
            )],
        )
        .unwrap();
        ensure_meta(&project, &session_id);

        let mut conversation = ConversationState::new(session_id.to_string());
        conversation
            .messages
            .push(AgentMessage::user("in-memory conversation".to_string()));
        conversation.current_turn = 7;
        crate::agent_engine::session_state::global().remove_session(&session_id);
        crate::agent_engine::session_state::global().save_conversation(&session_id, conversation);

        let output = hydrate(&project, &session_id);
        assert_eq!(output.hydration_status, "memory_hit");
        assert_eq!(output.runtime_state, "ready");
        assert!(output.can_continue);
        assert!(!output.can_resume);
        assert_eq!(output.last_turn, Some(7));
        assert_eq!(output.next_turn_id, Some(8));

        crate::agent_engine::session_state::global().remove_session(&session_id);
    }

    #[test]
    fn test_cleanup_stale_keeps_hydrate_capability_via_snapshot() {
        let project = setup_temp_project();
        let session_id = "test_cleanup_then_hydrate";
        let stream_path = session_stream_path(&project, session_id);

        append_events_jsonl(
            &stream_path,
            &[make_event(session_event_types::TURN_STARTED, session_id, 1)],
        )
        .unwrap();
        ensure_meta(&project, session_id);

        let mut conversation = ConversationState::new(session_id.to_string());
        conversation
            .messages
            .push(AgentMessage::user("persisted before cleanup".to_string()));
        conversation.current_turn = 3;

        save_runtime_snapshot_from_input(
            project.as_path(),
            RuntimeSnapshotUpsertInput::from_conversation(
                session_id.to_string(),
                SessionRuntimeState::Completed,
                conversation,
                Some(3),
                Some("openai-compatible".to_string()),
                Some("gpt-4o-mini".to_string()),
                Some("https://api.openai.com/v1".to_string()),
                None,
                None,
                None,
            ),
        )
        .unwrap();

        crate::agent_engine::session_state::global().remove_session(session_id);

        let output = hydrate(&project, session_id);
        assert_eq!(output.hydration_status, "snapshot_loaded");
        assert_eq!(output.runtime_state, "completed");
        assert!(output.can_continue);
        assert!(!output.can_resume);
        assert_eq!(output.next_turn_id, Some(4));

        crate::agent_engine::session_state::global().remove_session(session_id);
    }

    #[test]
    fn test_delete_session_removes_runtime_snapshot() {
        let project = setup_temp_project();
        let session_id = "test_delete_runtime_snapshot";
        let stream_path = session_stream_path(&project, session_id);

        append_events_jsonl(
            &stream_path,
            &[make_event(session_event_types::TURN_STARTED, session_id, 1)],
        )
        .unwrap();
        ensure_meta(&project, session_id);

        save_runtime_snapshot_from_input(
            project.as_path(),
            RuntimeSnapshotUpsertInput::readonly(
                session_id.to_string(),
                crate::application::command_usecases::agent_session::AgentSessionReadonlyReason::RuntimeStateUnavailable
                    .as_str()
                    .to_string(),
                Some(1),
            ),
        )
        .unwrap();

        let snapshot_path = runtime_snapshot_path(project.as_path(), session_id);
        assert!(snapshot_path.exists());

        delete_session(project.as_path(), session_id).unwrap();

        assert!(!snapshot_path.exists());
        assert!(!session_stream_path(&project, session_id).exists());
    }

    #[test]
    fn test_append_session_events_rejects_non_monotonic_seq() {
        let project = setup_temp_project();
        let session_id = "test_non_monotonic_seq";

        let first = make_event(session_event_types::TURN_STARTED, session_id, 1);
        append_session_events(project.as_path(), session_id, &[first]).unwrap();

        let second = make_event(session_event_types::TURN_COMPLETED, session_id, 1);
        let err = append_session_events(project.as_path(), session_id, &[second]).unwrap_err();

        assert!(matches!(err.code, crate::models::ErrorCode::Conflict));
        assert_eq!(
            err.details
                .as_ref()
                .and_then(|details| details.get("code"))
                .and_then(|value| value.as_str()),
            Some("E_AGENT_SESSION_EVENT_SEQ_NON_MONOTONIC")
        );

        let loaded = read_events_jsonl(&session_stream_path(&project, session_id)).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_append_session_events_dedupes_by_dedupe_key() {
        let project = setup_temp_project();
        let session_id = "test_dedupe_key";

        let mut first = make_event(session_event_types::TURN_STARTED, session_id, 1);
        first.dedupe_key = Some("turn_started:turn:1".to_string());

        let mut duplicate = make_event(session_event_types::TURN_STARTED, session_id, 2);
        duplicate.dedupe_key = Some("turn_started:turn:1".to_string());

        let first_result = append_session_events(project.as_path(), session_id, &[first]).unwrap();
        assert_eq!(first_result.appended_count, 1);

        let second_result =
            append_session_events(project.as_path(), session_id, &[duplicate]).unwrap();
        assert_eq!(second_result.appended_count, 0);
        assert_eq!(second_result.deduped_count, 1);

        let loaded = read_events_jsonl(&session_stream_path(&project, session_id)).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_append_session_events_updates_index_metadata() {
        let project = setup_temp_project();
        let session_id = "test_index_update";

        let event = AgentSessionEvent {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            event_type: session_event_types::TURN_COMPLETED.to_string(),
            session_id: session_id.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some("evt_index_update".to_string()),
            event_seq: Some(1),
            dedupe_key: Some("turn_completed:turn:3".to_string()),
            turn: Some(3),
            payload: Some(json!({
                "stop_reason": "success",
                "active_chapter_path": "chapters/ch3.md",
            })),
        };

        append_session_events(project.as_path(), session_id, &[event]).unwrap();

        let index = load_index(&session_index_path(project.as_path())).unwrap();
        let meta = find_meta(&index, session_id).expect("session meta should be created");

        assert_eq!(meta.last_turn, Some(3));
        assert_eq!(meta.last_stop_reason.as_deref(), Some("success"));
        assert_eq!(meta.active_chapter_path.as_deref(), Some("chapters/ch3.md"));
    }

    #[test]
    fn test_missing_meta_is_recovered_from_persisted_files() {
        let project = setup_temp_project();
        let session_id = "test_recover_missing_meta";

        let start_event = AgentSessionEvent {
            schema_version: AGENT_SESSION_SCHEMA_VERSION,
            event_type: session_event_types::SESSION_START.to_string(),
            session_id: session_id.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            event_id: Some(format!("evt_{}", uuid::Uuid::new_v4())),
            event_seq: Some(1),
            dedupe_key: Some("session_start".to_string()),
            turn: None,
            payload: Some(json!({
                "project_path": project.to_string_lossy().to_string(),
                "active_chapter_path": "manuscripts/ch1.json",
            })),
        };

        append_events_jsonl(&session_stream_path(&project, session_id), &[start_event]).unwrap();
        ensure_meta(&project, session_id);

        save_runtime_snapshot_from_input(
            project.as_path(),
            RuntimeSnapshotUpsertInput::readonly(
                session_id.to_string(),
                "recovery_test".to_string(),
                Some(3),
            ),
        )
        .unwrap();

        let index_path = session_index_path(project.as_path());
        let mut index = load_index(&index_path).unwrap();
        index.sessions.clear();
        save_index(&index_path, &index).unwrap();

        let recovered = load_session_meta_from_support(project.as_path(), session_id)
            .unwrap()
            .expect("recovered meta");
        assert_eq!(recovered.session_id, session_id);
        assert_eq!(recovered.last_turn, Some(3));
        assert_eq!(
            recovered.active_chapter_path.as_deref(),
            Some("manuscripts/ch1.json")
        );

        let persisted_index = load_index(&index_path).unwrap();
        assert!(
            find_meta(&persisted_index, session_id).is_some(),
            "recovered meta should be written back to index"
        );
    }
}
