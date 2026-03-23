//! Prompt Assembler — five-layer composable system prompt framework.
//!
//! Layers (assembled in order):
//!   A. Core   — fixed identity & behavioral rules (frozen, never modified at runtime)
//!   B. Mode   — interactive / exec / spec constraints
//!   C. Role   — orchestrator / context / draft / review / knowledge templates
//!   D. Reminder — dynamic runtime fields injected by DevE/DevC (pluggable)
//!   E. Patch  — provider/model-specific adjustments

pub mod core_layer;
pub mod mode_layer;
pub mod patch_layer;
pub mod reminder_layer;
pub mod role_layer;

#[cfg(test)]
mod tests;

use mode_layer::PromptMode;
use patch_layer::ModelPatch;
use reminder_layer::ReminderText;
use role_layer::PromptRole;

/// Assembled prompt output with per-layer visibility for debugging.
#[derive(Debug, Clone)]
pub struct AssembledPrompt {
    /// The final concatenated system prompt text.
    pub text: String,
    /// Per-layer segments (for debug / test inspection).
    pub segments: Vec<PromptSegment>,
}

#[derive(Debug, Clone)]
pub struct PromptSegment {
    pub layer: &'static str,
    pub content: String,
}

/// Builder that composes the five layers into a single system prompt.
#[derive(Debug, Clone)]
pub struct PromptAssembler {
    mode: PromptMode,
    role: Option<PromptRole>,
    reminder: Option<ReminderText>,
    patch: Option<ModelPatch>,
}

impl PromptAssembler {
    /// Create an assembler with the given mode. Core layer is always included.
    pub fn new(mode: PromptMode) -> Self {
        Self {
            mode,
            role: None,
            reminder: None,
            patch: None,
        }
    }

    pub fn with_role(mut self, role: PromptRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Set the reminder text (Layer D). Provided by DevE's reminder builder.
    pub fn with_reminder(mut self, reminder: ReminderText) -> Self {
        self.reminder = Some(reminder);
        self
    }

    pub fn with_patch(mut self, patch: ModelPatch) -> Self {
        self.patch = Some(patch);
        self
    }

    /// Assemble the final system prompt by concatenating all layers.
    pub fn assemble(&self) -> AssembledPrompt {
        let mut segments = Vec::new();

        // Layer A: Core (always present, frozen)
        let core = core_layer::render_core();
        segments.push(PromptSegment {
            layer: "core",
            content: core,
        });

        // Layer B: Mode
        let mode_text = mode_layer::render_mode(&self.mode);
        if !mode_text.is_empty() {
            segments.push(PromptSegment {
                layer: "mode",
                content: mode_text,
            });
        }

        // Layer C: Role
        if let Some(ref role) = self.role {
            let role_text = role_layer::render_role(role);
            if !role_text.is_empty() {
                segments.push(PromptSegment {
                    layer: "role",
                    content: role_text,
                });
            }
        }

        // Layer D: Reminder (pluggable injection point for DevE/DevC)
        if let Some(ref reminder) = self.reminder {
            let reminder_text = reminder_layer::render_reminder(reminder);
            if !reminder_text.is_empty() {
                segments.push(PromptSegment {
                    layer: "reminder",
                    content: reminder_text,
                });
            }
        }

        // Layer E: Model Patch
        if let Some(ref patch) = self.patch {
            let patch_text = patch_layer::render_patch(patch);
            if !patch_text.is_empty() {
                segments.push(PromptSegment {
                    layer: "patch",
                    content: patch_text,
                });
            }
        }

        let text = segments
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        AssembledPrompt { text, segments }
    }

    /// Validate the assembled prompt for internal consistency.
    /// Returns a list of warnings (empty = valid).
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let assembled = self.assemble();

        // spec mode must not instruct use of write tools in dynamic layers (mode/reminder/patch).
        // Core is excluded because it legitimately references tool names in its reference table.
        if self.mode == PromptMode::Spec {
            let write_tools = ["draft_write", "structure_edit", "knowledge_write"];
            // Check only the non-core segments (mode, role, reminder, patch)
            let dynamic_text: String = assembled
                .segments
                .iter()
                .filter(|s| s.layer != "core")
                .map(|s| s.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            for tool in &write_tools {
                // Flag only affirmative instructions ("use <tool>", "call <tool>").
                // Prohibitions like "Do NOT use <tool>" are acceptable and must not trigger.
                let affirmative_patterns = [
                    format!("- use {}", tool),
                    format!(". use {}", tool),
                    format!("call {}", tool),
                    format!("use {} to", tool),
                    format!("use {} for", tool),
                ];
                let lower = dynamic_text.to_lowercase();
                if affirmative_patterns
                    .iter()
                    .any(|p| lower.contains(p.as_str()))
                {
                    warnings.push(format!(
                        "spec mode prompt instructs use of write tool '{}'",
                        tool
                    ));
                }
            }
        }

        // Role forbidden constraints: check role doesn't leak cross-role tools
        if let Some(ref role) = self.role {
            for violation in role_layer::check_forbidden_violations(role, &assembled.text) {
                warnings.push(violation);
            }
        }

        warnings
    }
}
