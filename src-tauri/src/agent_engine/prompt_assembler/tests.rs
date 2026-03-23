//! Unit tests for PromptAssembler — five-layer system prompt framework.

use crate::agent_engine::types::{AgentMode, ClarificationMode};

use super::{
    mode_layer::{render_mode, PromptMode},
    patch_layer::{render_patch, ModelPatch},
    reminder_layer::{render_reminder, ReminderText},
    role_layer::{check_forbidden_violations, render_role, PromptRole},
    PromptAssembler,
};

// ---------------------------------------------------------------------------
// Layer A: Core
// ---------------------------------------------------------------------------

#[test]
fn core_layer_is_non_empty() {
    use super::core_layer::render_core;
    let core = render_core();
    assert!(!core.is_empty(), "core layer must not be empty");
    assert!(
        core.contains("Magic Novel AI"),
        "core layer must contain identity text"
    );
}

#[test]
fn core_layer_references_new_tools() {
    use super::core_layer::render_core;
    let core = render_core();
    assert!(
        core.contains("## Available tools"),
        "core must contain tools reference section"
    );
    for tool in [
        "workspace_map",
        "context_read",
        "context_search",
        "knowledge_read",
        "knowledge_write",
        "draft_write",
        "structure_edit",
        "review_check",
        "askuser",
        "todowrite",
    ] {
        assert!(core.contains(tool), "core must reference tool '{}'", tool);
    }
}

#[test]
fn core_layer_does_not_reference_legacy_tools_or_protocol() {
    use super::core_layer::render_core;
    let core = render_core();

    for legacy_tool in [
        "| read |",
        "| edit |",
        "| create |",
        "| delete |",
        "| move |",
        "| ls |",
        "| grep |",
        "| outline |",
        "| character_sheet |",
        "| search_knowledge |",
    ] {
        assert!(
            !core.contains(legacy_tool),
            "core must not reference legacy tool '{}'",
            legacy_tool
        );
    }

    for legacy_keyword in [
        "snapshot_id",
        "base_revision",
        "replace_range",
        "## Edit workflow",
    ] {
        assert!(
            !core.contains(legacy_keyword),
            "core must not reference legacy edit protocol keyword '{}'",
            legacy_keyword
        );
    }
}

#[test]
fn core_layer_does_not_contain_mode_keywords() {
    use super::core_layer::render_core;
    let core = render_core();
    // Mode text like "## Mode: spec" must not bleed into core
    assert!(!core.contains("## Mode:"), "core must not embed mode text");
}

// ---------------------------------------------------------------------------
// Layer B: Mode
// ---------------------------------------------------------------------------

#[test]
fn mode_from_planning_is_spec() {
    let mode = PromptMode::from_engine_modes(AgentMode::Planning, ClarificationMode::Interactive);
    assert_eq!(mode, PromptMode::Spec);
}

#[test]
fn mode_from_writing_interactive_is_interactive() {
    let mode = PromptMode::from_engine_modes(AgentMode::Writing, ClarificationMode::Interactive);
    assert_eq!(mode, PromptMode::Interactive);
}

#[test]
fn mode_from_writing_headless_is_exec() {
    let mode = PromptMode::from_engine_modes(AgentMode::Writing, ClarificationMode::HeadlessDefer);
    assert_eq!(mode, PromptMode::Exec);
}

#[test]
fn spec_mode_text_forbids_write_tools() {
    let text = render_mode(&PromptMode::Spec);
    assert!(text.contains("MUST NOT") || text.contains("Do NOT"));
    assert!(text.contains("draft_write"));
    assert!(text.contains("structure_edit"));
    assert!(text.contains("knowledge_write"));
}

#[test]
fn exec_mode_text_disallows_askuser() {
    let text = render_mode(&PromptMode::Exec);
    assert!(text.contains("Do NOT call askuser") || text.contains("Do NOT\ncall askuser"));
}

#[test]
fn interactive_mode_text_mentions_askuser() {
    let text = render_mode(&PromptMode::Interactive);
    assert!(text.contains("askuser"));
}

// ---------------------------------------------------------------------------
// Layer C: Role
// ---------------------------------------------------------------------------

#[test]
fn review_role_forbids_draft_write() {
    assert!(PromptRole::Review
        .forbidden_tools()
        .contains(&"draft_write"));
}

#[test]
fn draft_role_forbids_knowledge_write() {
    assert!(PromptRole::Draft
        .forbidden_tools()
        .contains(&"knowledge_write"));
}

#[test]
fn knowledge_role_forbids_draft_write() {
    assert!(PromptRole::Knowledge
        .forbidden_tools()
        .contains(&"draft_write"));
}

#[test]
fn review_role_text_does_not_instruct_draft_write() {
    let text = render_role(&PromptRole::Review);
    let violations = check_forbidden_violations(&PromptRole::Review, &text);
    assert!(
        violations.is_empty(),
        "review role text must not instruct forbidden tools: {:?}",
        violations
    );
}

#[test]
fn knowledge_role_text_does_not_instruct_draft_write() {
    let text = render_role(&PromptRole::Knowledge);
    let violations = check_forbidden_violations(&PromptRole::Knowledge, &text);
    assert!(
        violations.is_empty(),
        "knowledge role text must not instruct forbidden tools: {:?}",
        violations
    );
}

#[test]
fn context_role_text_prohibitions_do_not_trigger_violations() {
    let text = render_role(&PromptRole::Context);
    let violations = check_forbidden_violations(&PromptRole::Context, &text);
    assert!(
        violations.is_empty(),
        "context role text contains prohibitions like 'Do NOT use ...' and must not trigger: {:?}",
        violations
    );
}

#[test]
fn role_switching_does_not_change_core() {
    use super::core_layer::render_core;
    let core = render_core();
    let review_text = render_role(&PromptRole::Review);
    let draft_text = render_role(&PromptRole::Draft);
    // Role text is separate from core
    assert_ne!(core, review_text);
    assert_ne!(core, draft_text);
    // Core content is stable regardless of role
    assert_eq!(render_core(), core);
}

// ---------------------------------------------------------------------------
// Layer D: Reminder
// ---------------------------------------------------------------------------

#[test]
fn reminder_wraps_in_system_reminder_tags() {
    let r = ReminderText::new("Mode: interactive\nScope: chapter:vol1/ch1.json");
    let rendered = render_reminder(&r);
    assert!(rendered.starts_with("<system-reminder>"));
    assert!(rendered.ends_with("</system-reminder>"));
    assert!(rendered.contains("Mode: interactive"));
}

#[test]
fn empty_reminder_renders_to_empty_string() {
    let r = ReminderText::new("");
    assert!(render_reminder(&r).is_empty());
}

#[test]
fn whitespace_only_reminder_renders_to_empty_string() {
    let r = ReminderText::new("   \n  ");
    assert!(render_reminder(&r).is_empty());
}

// ---------------------------------------------------------------------------
// Layer E: Patch
// ---------------------------------------------------------------------------

#[test]
fn openai_patch_is_non_empty() {
    let patch = ModelPatch::new("openai-compatible", "gpt-4o");
    assert!(!render_patch(&patch).is_empty());
}

#[test]
fn unknown_provider_patch_is_empty() {
    let patch = ModelPatch::new("unknown-provider", "some-model");
    assert!(render_patch(&patch).is_empty());
}

#[test]
fn patch_does_not_modify_core() {
    use super::core_layer::render_core;
    let core_before = render_core();
    let _patch = render_patch(&ModelPatch::new("openai-compatible", "gpt-4o"));
    let core_after = render_core();
    assert_eq!(core_before, core_after);
}

// ---------------------------------------------------------------------------
// PromptAssembler integration
// ---------------------------------------------------------------------------

#[test]
fn assemble_with_only_mode_includes_core_and_mode() {
    let assembled = PromptAssembler::new(PromptMode::Interactive).assemble();
    let layers: Vec<&str> = assembled.segments.iter().map(|s| s.layer).collect();
    assert!(layers.contains(&"core"));
    assert!(layers.contains(&"mode"));
    assert!(!layers.contains(&"role"));
    assert!(!layers.contains(&"reminder"));
    assert!(!layers.contains(&"patch"));
}

#[test]
fn replacing_mode_does_not_affect_core_segment() {
    let a1 = PromptAssembler::new(PromptMode::Interactive).assemble();
    let a2 = PromptAssembler::new(PromptMode::Exec).assemble();
    let core1 = a1.segments.iter().find(|s| s.layer == "core").unwrap();
    let core2 = a2.segments.iter().find(|s| s.layer == "core").unwrap();
    assert_eq!(
        core1.content, core2.content,
        "core must be identical across modes"
    );
}

#[test]
fn replacing_patch_does_not_affect_core_segment() {
    let a1 = PromptAssembler::new(PromptMode::Interactive)
        .with_patch(ModelPatch::new("openai-compatible", "gpt-4o"))
        .assemble();
    let a2 = PromptAssembler::new(PromptMode::Interactive).assemble();
    let core1 = a1.segments.iter().find(|s| s.layer == "core").unwrap();
    let core2 = a2.segments.iter().find(|s| s.layer == "core").unwrap();
    assert_eq!(
        core1.content, core2.content,
        "core must be identical with/without patch"
    );
}

#[test]
fn role_segment_is_independent_from_mode_segment() {
    let assembled = PromptAssembler::new(PromptMode::Exec)
        .with_role(PromptRole::Review)
        .assemble();
    let mode_seg = assembled
        .segments
        .iter()
        .find(|s| s.layer == "mode")
        .unwrap();
    let role_seg = assembled
        .segments
        .iter()
        .find(|s| s.layer == "role")
        .unwrap();
    assert_ne!(mode_seg.content, role_seg.content);
}

#[test]
fn reminder_segment_is_present_when_set() {
    let reminder = ReminderText::new("Mode: exec\nScope: chapter:vol1/ch1.json");
    let assembled = PromptAssembler::new(PromptMode::Exec)
        .with_reminder(reminder)
        .assemble();
    let reminder_seg = assembled
        .segments
        .iter()
        .find(|s| s.layer == "reminder")
        .expect("reminder segment must be present");
    assert!(reminder_seg.content.contains("<system-reminder>"));
    assert!(reminder_seg.content.contains("Mode: exec"));
}

#[test]
fn spec_mode_validate_warns_on_write_tool_in_assembled_text() {
    // Inject a fake "reminder" that accidentally mentions a write tool instruction
    let bad_reminder = ReminderText::new("Please use draft_write to continue.");
    let warnings = PromptAssembler::new(PromptMode::Spec)
        .with_reminder(bad_reminder)
        .validate();
    assert!(
        !warnings.is_empty(),
        "spec mode with write tool mention must produce warnings"
    );
}

#[test]
fn spec_mode_validate_clean_produces_no_warnings() {
    let warnings = PromptAssembler::new(PromptMode::Spec).validate();
    assert!(
        warnings.is_empty(),
        "spec mode without reminder must be clean: {:?}",
        warnings
    );
}

#[test]
fn layer_order_is_core_mode_role_reminder_patch() {
    let assembled = PromptAssembler::new(PromptMode::Interactive)
        .with_role(PromptRole::Draft)
        .with_reminder(ReminderText::new("Mode: interactive"))
        .with_patch(ModelPatch::new("openai-compatible", "gpt-4o"))
        .assemble();
    let layers: Vec<&str> = assembled.segments.iter().map(|s| s.layer).collect();
    assert_eq!(layers, vec!["core", "mode", "role", "reminder", "patch"]);
}
