//! Macro workflow types (v0 frozen contract)
//!
//! Aligned with worktrees/docs/M5/guide.md §2.2–2.3.
//! Add-only field policy: never remove fields, only append with `#[serde(default)]`.

use serde::{Deserialize, Serialize};

pub const MACRO_SCHEMA_VERSION: i32 = 1;

// ── WorkflowKind ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowKind {
    Book,
    Volume,
}

// ── TokenBudget ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TokenBudget {
    Small,
    Medium,
    Large,
}

// ── MacroStage ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MacroStage {
    Planning,
    Context,
    Draft,
    Review,
    Fix,
    Writeback,
    Integrate,
    Completed,
    Blocked,
    Failed,
    Cancelled,
}

// ── ChapterTarget (config input) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterTarget {
    pub chapter_ref: String,
    pub write_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
}

// ── MacroWorkflowConfig (macro/config.json — immutable) ────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroWorkflowConfig {
    pub schema_version: i32,
    pub macro_id: String,
    pub mission_id: String,
    pub workflow_kind: WorkflowKind,
    pub objective: String,
    pub chapter_targets: Vec<ChapterTarget>,
    pub strict_review: bool,
    pub auto_fix_on_block: bool,
    pub token_budget: TokenBudget,
    pub created_at: i64,
}

// ── ChapterRunStatus ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterRunStatus {
    Pending,
    Running,
    Completed,
    Blocked,
    Failed,
    Skipped,
}

// ── ChapterRunState ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterRunState {
    pub chapter_ref: String,
    pub write_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,

    pub status: ChapterRunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<MacroStage>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_contextpack_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_review_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_knowledge_delta_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_handoff_summary: Option<String>,

    pub updated_at: i64,
}

// ── MacroLastError ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroLastError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
}

// ── MacroWorkflowState (macro/state.json — mutable) ────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroWorkflowState {
    pub schema_version: i32,
    pub macro_id: String,
    pub mission_id: String,
    pub objective: String,
    pub workflow_kind: WorkflowKind,

    pub current_index: i32,
    pub current_stage: MacroStage,

    pub chapters: Vec<ChapterRunState>,

    pub last_transition_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<MacroLastError>,
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> MacroWorkflowConfig {
        MacroWorkflowConfig {
            schema_version: MACRO_SCHEMA_VERSION,
            macro_id: "macro_001".into(),
            mission_id: "mis_abc".into(),
            workflow_kind: WorkflowKind::Book,
            objective: "Advance 3 chapters".into(),
            chapter_targets: vec![
                ChapterTarget {
                    chapter_ref: "vol1/ch1".into(),
                    write_path: "chapters/ch1.md".into(),
                    display_title: Some("Chapter 1".into()),
                },
                ChapterTarget {
                    chapter_ref: "vol1/ch2".into(),
                    write_path: "chapters/ch2.md".into(),
                    display_title: None,
                },
            ],
            strict_review: false,
            auto_fix_on_block: true,
            token_budget: TokenBudget::Medium,
            created_at: 1700000000000,
        }
    }

    fn sample_state() -> MacroWorkflowState {
        let now = 1700000000000i64;
        MacroWorkflowState {
            schema_version: MACRO_SCHEMA_VERSION,
            macro_id: "macro_001".into(),
            mission_id: "mis_abc".into(),
            objective: "Advance 3 chapters".into(),
            workflow_kind: WorkflowKind::Book,
            current_index: -1,
            current_stage: MacroStage::Planning,
            chapters: vec![ChapterRunState {
                chapter_ref: "vol1/ch1".into(),
                write_path: "chapters/ch1.md".into(),
                display_title: Some("Chapter 1".into()),
                status: ChapterRunStatus::Pending,
                stage: None,
                latest_contextpack_ref: None,
                latest_review_id: None,
                latest_knowledge_delta_id: None,
                last_handoff_summary: None,
                updated_at: now,
            }],
            last_transition_at: now,
            last_error: None,
        }
    }

    #[test]
    fn config_serde_roundtrip() {
        let cfg = sample_config();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let parsed: MacroWorkflowConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.macro_id, "macro_001");
        assert_eq!(parsed.chapter_targets.len(), 2);
        assert_eq!(parsed.workflow_kind, WorkflowKind::Book);
        assert_eq!(parsed.token_budget, TokenBudget::Medium);
    }

    #[test]
    fn state_serde_roundtrip() {
        let st = sample_state();
        let json = serde_json::to_string_pretty(&st).unwrap();
        let parsed: MacroWorkflowState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.current_index, -1);
        assert_eq!(parsed.current_stage, MacroStage::Planning);
        assert_eq!(parsed.chapters.len(), 1);
        assert_eq!(parsed.chapters[0].status, ChapterRunStatus::Pending);
    }

    #[test]
    fn state_with_error_roundtrip() {
        let mut st = sample_state();
        st.last_error = Some(MacroLastError {
            code: "E_REVIEW_BLOCKED".into(),
            message: "review blocked on ch1".into(),
            feature_id: Some("ch1_draft".into()),
            worker_id: None,
        });
        let json = serde_json::to_string(&st).unwrap();
        let parsed: MacroWorkflowState = serde_json::from_str(&json).unwrap();
        let err = parsed.last_error.unwrap();
        assert_eq!(err.code, "E_REVIEW_BLOCKED");
        assert!(err.worker_id.is_none());
    }

    #[test]
    fn stage_snake_case_serde() {
        let json = serde_json::to_string(&MacroStage::Writeback).unwrap();
        assert_eq!(json, "\"writeback\"");
        let parsed: MacroStage = serde_json::from_str("\"context\"").unwrap();
        assert_eq!(parsed, MacroStage::Context);
    }
}
