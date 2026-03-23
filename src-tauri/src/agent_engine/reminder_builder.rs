//! DevE: Dynamic Reminder Builder (Prompt-P3)
//!
//! Builds the six-field `<system-reminder>` block injected at session
//! start, resume, and scope-change turns.
//!
//! Fields (plan_03 spec):
//!   Mode          — from PromptMode
//!   Scope         — active_chapter_path or "global"
//!   Session       — "new" | "resume"
//!   Branch        — active branch_id from branch_state.json
//!   Pending Todos — count of pending/in_progress items from latest todowrite
//!   Canon Version — accepted@{revision} from canon_version.json
//!
//! Drift warnings (max 3 lines):
//!   - canon drift  : reminder was built with a different revision than current
//!   - pending block: pending review blocker signal from DevC
//!   - branch drift : branch changed since session start
//!
//! Token budget: target < 200 tokens (enforced by line/field caps).

use std::path::Path;

use crate::agent_engine::prompt_assembler::mode_layer::PromptMode;
use crate::agent_engine::prompt_assembler::reminder_layer::ReminderText;
use crate::gate_integration::read_canon_version;
use crate::services::knowledge_paths::resolve_knowledge_root_for_read;

const BRANCH_STATE_FILE: &str = "branch_state.json";
const DEFAULT_BRANCH_ID: &str = "branch/main";

/// Whether this is a new session start or a resume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    New,
    Resume,
}

impl SessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Resume => "resume",
        }
    }
}

/// Input for building the dynamic reminder.
#[derive(Debug, Clone)]
pub struct ReminderInput<'a> {
    /// Absolute path to the project root (used to read canon_version + branch_state).
    pub project_path: &'a Path,
    /// Prompt mode derived from LoopConfig.
    pub mode: PromptMode,
    /// Active chapter path (manuscripts-relative) or None for global scope.
    pub active_chapter_path: Option<&'a str>,
    /// Whether this is a new session or a resume.
    pub session_kind: SessionKind,
    /// Pending todo count: pending + in_progress items from the most recent todowrite.
    pub pending_todo_count: Option<usize>,
    /// Optional pre-rendered Active Rules summary from DevC.
    pub active_rules_summary: Option<&'a str>,
    /// Pending-blocker signal from DevC: true if there is an unresolved block-level review.
    pub has_pending_blocker: bool,
    /// Canon revision that was in effect when this session started (for drift detection).
    pub session_canon_revision: Option<i64>,
    /// Branch id that was active when this session started (for drift detection).
    pub session_branch_id: Option<&'a str>,
}

impl<'a> ReminderInput<'a> {
    pub fn new(project_path: &'a Path, mode: PromptMode, session_kind: SessionKind) -> Self {
        Self {
            project_path,
            mode,
            active_chapter_path: None,
            session_kind,
            pending_todo_count: None,
            active_rules_summary: None,
            has_pending_blocker: false,
            session_canon_revision: None,
            session_branch_id: None,
        }
    }
}

/// Read the active branch_id from `branch_state.json`.
/// Falls back to `"branch/main"` on any error.
pub(crate) fn read_active_branch_id(project_path: &Path) -> String {
    let path = resolve_knowledge_root_for_read(project_path).join(BRANCH_STATE_FILE);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return DEFAULT_BRANCH_ID.to_string();
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
        if let Some(id) = v.get("active_branch_id").and_then(|v| v.as_str()) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    DEFAULT_BRANCH_ID.to_string()
}

/// Build the `ReminderText` for Layer D of PromptAssembler.
pub fn build_reminder(input: &ReminderInput<'_>) -> ReminderText {
    let mut lines: Vec<String> = Vec::with_capacity(12);

    // Six standard fields
    lines.push(format!("Mode: {}", input.mode.as_str()));

    let scope = match input.active_chapter_path {
        Some(p) if !p.trim().is_empty() => {
            if p.contains(':') {
                p.to_string()
            } else {
                format!("chapter:{}", p)
            }
        }
        _ => "global".to_string(),
    };
    lines.push(format!("Scope: {}", scope));

    lines.push(format!("Session: {}", input.session_kind.as_str()));

    let branch_id = read_active_branch_id(input.project_path);
    lines.push(format!("Branch: {}", branch_id));

    let pending = input.pending_todo_count.unwrap_or(0);
    lines.push(format!("Pending Todos: {}", pending));

    let canon_revision = read_canon_version(input.project_path)
        .ok()
        .flatten()
        .map(|cv| cv.revision);
    let canon_str = canon_revision
        .map(|r| format!("accepted@{}", r))
        .unwrap_or_else(|| "none".to_string());
    lines.push(format!("Canon Version: {}", canon_str));

    // Drift warnings (max 3)
    let mut warnings: Vec<String> = Vec::new();

    if let (Some(session_rev), Some(current_rev)) = (input.session_canon_revision, canon_revision) {
        if session_rev != current_rev {
            warnings.push(format!(
                "WARN canon_drift: session started at accepted@{} but current is accepted@{} — re-read knowledge before writing.",
                session_rev, current_rev
            ));
        }
    }

    if input.has_pending_blocker {
        warnings.push(
            "WARN pending_blocker: unresolved block-level review exists — fix or skip before proceeding."
                .to_string(),
        );
    }

    if let Some(session_branch) = input.session_branch_id {
        if session_branch.trim() != branch_id.as_str() {
            warnings.push(format!(
                "WARN branch_drift: session started on '{}' but active branch is now '{}' — check before writing.",
                session_branch.trim(),
                branch_id
            ));
        }
    }

    for w in warnings.into_iter().take(3) {
        lines.push(w);
    }

    // Active Rules summary (optional)
    if let Some(summary) = input.active_rules_summary {
        let trimmed = summary.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }

    ReminderText::new(lines.join("\n"))
}
