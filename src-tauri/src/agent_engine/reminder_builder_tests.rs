//! Unit tests for reminder_builder (DevE — Prompt-P3)

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::agent_engine::prompt_assembler::mode_layer::PromptMode;
    use crate::agent_engine::reminder_builder::{build_reminder, ReminderInput, SessionKind};

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("reminder_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn write_canon_version(project: &PathBuf, revision: i64, branch_id: &str) {
        let meta = project.join(".magic_novel").join("_meta");
        fs::create_dir_all(&meta).unwrap();
        let cv = serde_json::json!({
            "schema_version": 1,
            "branch_id": branch_id,
            "revision": revision,
            "updated_at": 0i64,
        });
        fs::write(
            meta.join("canon_version.json"),
            serde_json::to_string(&cv).unwrap(),
        )
        .unwrap();
    }

    fn write_branch_state(project: &PathBuf, branch_id: &str) {
        // branch_state.json lives directly under the knowledge root.
        // In the default layout the knowledge root is `{project}/.magic_novel/`.
        let knowledge_root = project.join(".magic_novel");
        fs::create_dir_all(&knowledge_root).unwrap();
        let doc = serde_json::json!({
            "schema_version": 1,
            "active_branch_id": branch_id,
            "updated_at": 0i64,
        });
        fs::write(
            knowledge_root.join("branch_state.json"),
            serde_json::to_string(&doc).unwrap(),
        )
        .unwrap();
    }

    // -----------------------------------------------------------------------
    // Six-field presence
    // -----------------------------------------------------------------------

    #[test]
    fn reminder_contains_all_six_fields_new_session() {
        let project = temp_project();
        let input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::New);
        let reminder = build_reminder(&input);
        let text = reminder.as_str();

        assert!(
            text.contains("Mode: interactive"),
            "missing Mode; got: {text}"
        );
        assert!(text.contains("Scope: global"), "missing Scope; got: {text}");
        assert!(
            text.contains("Session: new"),
            "missing Session; got: {text}"
        );
        assert!(text.contains("Branch:"), "missing Branch; got: {text}");
        assert!(
            text.contains("Pending Todos:"),
            "missing Pending Todos; got: {text}"
        );
        assert!(
            text.contains("Canon Version:"),
            "missing Canon Version; got: {text}"
        );

        cleanup(&project);
    }

    #[test]
    fn reminder_session_resume() {
        let project = temp_project();
        let input = ReminderInput::new(&project, PromptMode::Exec, SessionKind::Resume);
        let reminder = build_reminder(&input);
        assert!(reminder.as_str().contains("Session: resume"));
        cleanup(&project);
    }

    // -----------------------------------------------------------------------
    // Scope field
    // -----------------------------------------------------------------------

    #[test]
    fn scope_uses_chapter_path_when_set() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::New);
        input.active_chapter_path = Some("vol1/ch001.json");
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("Scope: chapter:vol1/ch001.json"),
            "got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn scope_already_prefixed_not_doubled() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::New);
        input.active_chapter_path = Some("chapter:vol1/ch001.json");
        let reminder = build_reminder(&input);
        let text = reminder.as_str();
        assert!(
            text.contains("Scope: chapter:vol1/ch001.json"),
            "got: {text}"
        );
        // Must not double the prefix
        assert!(
            !text.contains("Scope: chapter:chapter:"),
            "prefix doubled; got: {text}"
        );
        cleanup(&project);
    }

    // -----------------------------------------------------------------------
    // Branch field
    // -----------------------------------------------------------------------

    #[test]
    fn branch_uses_branch_state_when_present() {
        let project = temp_project();
        write_branch_state(&project, "branch/dev");
        let input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::Resume);
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("Branch: branch/dev"),
            "got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    // -----------------------------------------------------------------------
    // Canon Version field
    // -----------------------------------------------------------------------

    #[test]
    fn canon_version_reads_from_file() {
        let project = temp_project();
        write_canon_version(&project, 7, "branch/main");
        let input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::New);
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("Canon Version: accepted@7"),
            "got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn canon_version_none_when_file_missing() {
        let project = temp_project();
        let input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::New);
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("Canon Version: none"),
            "got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    // -----------------------------------------------------------------------
    // Pending todos
    // -----------------------------------------------------------------------

    #[test]
    fn pending_todos_count_reflected() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Exec, SessionKind::Resume);
        input.pending_todo_count = Some(3);
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("Pending Todos: 3"),
            "got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    // -----------------------------------------------------------------------
    // Token budget (< 200 tokens ≈ < 800 chars for typical prose)
    // -----------------------------------------------------------------------

    #[test]
    fn reminder_text_under_token_budget() {
        let project = temp_project();
        write_canon_version(&project, 1, "branch/main");
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::Resume);
        input.active_chapter_path = Some("vol1/ch001.json");
        input.pending_todo_count = Some(2);
        input.active_rules_summary = Some("[Active Rules: vol1/ch001.json]\n- words: 2000-3000");
        let reminder = build_reminder(&input);
        // 200 tokens ≈ 800 chars (conservative estimate for English/mixed text)
        assert!(
            reminder.as_str().len() < 800,
            "reminder too long ({} chars); got: {}",
            reminder.as_str().len(),
            reminder.as_str()
        );
        cleanup(&project);
    }

    // -----------------------------------------------------------------------
    // Drift warnings
    // -----------------------------------------------------------------------

    #[test]
    fn canon_drift_warning_when_revision_changed() {
        let project = temp_project();
        write_canon_version(&project, 5, "branch/main");
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::Resume);
        input.session_canon_revision = Some(3); // started at rev 3, now at 5
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("WARN canon_drift"),
            "expected canon_drift warning; got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn no_canon_drift_when_revision_unchanged() {
        let project = temp_project();
        write_canon_version(&project, 5, "branch/main");
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::Resume);
        input.session_canon_revision = Some(5);
        let reminder = build_reminder(&input);
        assert!(
            !reminder.as_str().contains("WARN canon_drift"),
            "unexpected canon_drift warning; got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn pending_blocker_warning_when_flagged() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Exec, SessionKind::Resume);
        input.has_pending_blocker = true;
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("WARN pending_blocker"),
            "expected pending_blocker warning; got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn branch_drift_warning_when_branch_changed() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::Resume);
        // branch_state.json absent -> defaults to branch/main
        input.session_branch_id = Some("branch/dev"); // started on dev, now main
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("WARN branch_drift"),
            "expected branch_drift warning; got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn no_branch_drift_when_branch_unchanged() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::Resume);
        input.session_branch_id = Some("branch/main");
        let reminder = build_reminder(&input);
        assert!(
            !reminder.as_str().contains("WARN branch_drift"),
            "unexpected branch_drift; got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn warnings_capped_at_three() {
        let project = temp_project();
        write_canon_version(&project, 5, "branch/main");
        let mut input = ReminderInput::new(&project, PromptMode::Exec, SessionKind::Resume);
        // Trigger all three warning types
        input.session_canon_revision = Some(1);
        input.has_pending_blocker = true;
        input.session_branch_id = Some("branch/dev");
        let reminder = build_reminder(&input);
        let warn_count = reminder
            .as_str()
            .lines()
            .filter(|l| l.starts_with("WARN "))
            .count();
        assert!(
            warn_count <= 3,
            "too many warnings ({warn_count}); got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    // -----------------------------------------------------------------------
    // Active Rules summary injection
    // -----------------------------------------------------------------------

    #[test]
    fn active_rules_summary_appended_when_provided() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::New);
        input.active_rules_summary = Some("[Active Rules: vol1/ch1.json]\n- words: 2000-3000");
        let reminder = build_reminder(&input);
        assert!(
            reminder.as_str().contains("[Active Rules:"),
            "expected rules summary; got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }

    #[test]
    fn empty_rules_summary_not_appended() {
        let project = temp_project();
        let mut input = ReminderInput::new(&project, PromptMode::Interactive, SessionKind::New);
        input.active_rules_summary = Some("   ");
        let reminder = build_reminder(&input);
        assert!(
            !reminder.as_str().contains("[Active Rules:"),
            "empty rules should not appear; got: {}",
            reminder.as_str()
        );
        cleanup(&project);
    }
}
