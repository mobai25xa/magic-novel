use crate::agent_engine::messages::{AgentMessage, ContentBlock, ConversationState, Role};
use crate::agent_engine::prompt_assembler::{
    mode_layer::PromptMode, patch_layer::ModelPatch, reminder_layer::ReminderText,
    role_layer::PromptRole, PromptAssembler,
};
use crate::agent_engine::types::LoopConfig;

#[cfg(test)]
pub(super) fn default_system_prompt() -> &'static str {
    crate::agent_engine::prompt_assembler::core_layer::render_core_static()
}

/// Build an assembled system prompt for the given loop config and optional provider/model.
/// Mode is derived from the loop config so spec / exec / interactive get distinct constraints.
/// An optional pre-built `ReminderText` (from DevE's reminder_builder) may be injected as Layer D.
pub(super) fn build_assembled_prompt(
    config: &LoopConfig,
    provider: Option<&str>,
    model: Option<&str>,
    role: Option<PromptRole>,
    reminder: Option<ReminderText>,
) -> String {
    let mode = PromptMode::from_engine_modes(config.capability_mode, config.clarification_mode);
    let mut assembler = PromptAssembler::new(mode);

    // Default role for user-facing interactive sessions. Worker loops should pass an explicit role.
    let effective_role = role.or_else(|| {
        if mode == PromptMode::Interactive {
            Some(PromptRole::Orchestrator)
        } else {
            None
        }
    });
    if let Some(r) = effective_role {
        assembler = assembler.with_role(r);
    }

    if let (Some(prov), Some(mdl)) = (provider, model) {
        assembler = assembler.with_patch(ModelPatch::new(prov, mdl));
    }
    if let Some(r) = reminder {
        assembler = assembler.with_reminder(r);
    }
    assembler.assemble().text
}

/// Insert or replace the system prompt, using the assembled mode-aware prompt when no
/// explicit override is provided.
/// `reminder` is an optional pre-built Layer D block from DevE's reminder_builder.
pub(super) fn apply_system_prompt_with_config(
    state: &mut ConversationState,
    system_prompt: Option<&str>,
    loop_config: &LoopConfig,
    provider: Option<&str>,
    model: Option<&str>,
    role: Option<PromptRole>,
    reminder: Option<ReminderText>,
) {
    let incoming = system_prompt.map(str::trim).filter(|s| !s.is_empty());
    let effective: String = if let Some(sys) = incoming {
        sys.to_string()
    } else {
        build_assembled_prompt(loop_config, provider, model, role, reminder)
    };

    if state.messages.is_empty() {
        state.messages.push(AgentMessage::system(effective));
        return;
    }
    let has_system_first = matches!(state.messages.first().map(|m| &m.role), Some(Role::System));
    if !has_system_first {
        state.messages.insert(0, AgentMessage::system(effective));
        return;
    }
    // Refresh on every start (mode may have changed between turns)
    if let Some(first) = state.messages.first_mut() {
        first.blocks = vec![ContentBlock::Text { text: effective }];
    }
}

#[cfg(test)]
pub(super) fn apply_system_prompt(state: &mut ConversationState, system_prompt: Option<&str>) {
    let incoming = system_prompt.map(str::trim).filter(|s| !s.is_empty());

    if state.messages.is_empty() {
        if let Some(sys) = incoming {
            state.messages.push(AgentMessage::system(sys.to_string()));
        }
    }

    let has_system_first = matches!(state.messages.first().map(|m| &m.role), Some(Role::System));

    if !has_system_first {
        let text = incoming.unwrap_or(default_system_prompt());
        state
            .messages
            .insert(0, AgentMessage::system(text.to_string()));
        return;
    }

    if let Some(sys) = incoming {
        if let Some(first) = state.messages.first_mut() {
            first.blocks = vec![ContentBlock::Text {
                text: sys.to_string(),
            }];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_default_includes_orchestrator_role() {
        let config = LoopConfig::default();
        let prompt = build_assembled_prompt(&config, None, None, None, None);
        assert!(
            prompt.contains("## Role: orchestrator"),
            "interactive sessions should inject orchestrator role by default"
        );
    }

    #[test]
    fn explicit_role_overrides_interactive_default() {
        let config = LoopConfig::default();
        let prompt = build_assembled_prompt(&config, None, None, Some(PromptRole::Draft), None);
        assert!(prompt.contains("## Role: draft"));
        assert!(!prompt.contains("## Role: orchestrator"));
    }
}
