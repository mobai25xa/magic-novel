//! Layer D: Reminder — dynamic runtime fields injected at session start/resume/scope-change.
//!
//! This layer is the stable injection point for DevE (reminder builder) and
//! DevC (gate signal bits). Neither module touches Layer A (Core).
//!
//! The `ReminderText` type is an opaque wrapper so callers cannot accidentally
//! place arbitrary content in Core or Mode layers.

/// Opaque reminder text produced by DevE's reminder builder.
///
/// Create via `ReminderText::new(text)`. The assembler wraps it in
/// `<system-reminder>` tags when rendering.
#[derive(Debug, Clone)]
pub struct ReminderText(String);

impl ReminderText {
    /// Create a reminder from pre-rendered text.
    /// The text should NOT include the `<system-reminder>` wrapper tags;
    /// `render_reminder()` adds them.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Access the inner text (for testing).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if the reminder text is empty or whitespace-only.
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

/// Render the reminder layer wrapped in standard tags.
///
/// Output format:
/// ```text
/// <system-reminder>
/// Mode: interactive
/// Scope: chapter:vol1/ch1.json
/// ...
/// </system-reminder>
/// ```
pub fn render_reminder(reminder: &ReminderText) -> String {
    if reminder.is_empty() {
        return String::new();
    }
    format!(
        "<system-reminder>\n{}\n</system-reminder>",
        reminder.as_str().trim()
    )
}
