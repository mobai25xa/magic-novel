//! Layer C: Role — orchestrator / context / draft / review / knowledge templates.
//!
//! Each role has:
//!   - A descriptive prompt explaining its responsibilities.
//!   - A set of Forbidden tools that must never appear in its output.
//!
//! Forbidden constraints are enforced via `check_forbidden_violations()`,
//! called by `PromptAssembler::validate()`.

/// Available role identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptRole {
    /// Top-level orchestrator: decomposes tasks and coordinates execution.
    Orchestrator,
    /// Context specialist: reads, searches, and synthesizes project knowledge.
    Context,
    /// Draft writer: writes and rewrites chapter content only.
    Draft,
    /// Reviewer: checks quality and consistency; never produces draft content.
    Review,
    /// Knowledge manager: proposes knowledge changes; never applies directly.
    Knowledge,
}

impl PromptRole {
    /// Tools this role must never invoke (hard forbidden).
    pub fn forbidden_tools(&self) -> &'static [&'static str] {
        match self {
            // Orchestrator coordinates work; must not directly write chapter text or mutate knowledge
            Self::Orchestrator => &["draft_write", "knowledge_write"],
            // Context role is read-only
            Self::Context => &["draft_write", "structure_edit", "knowledge_write"],
            // Draft writer must not touch knowledge or structure
            Self::Draft => &["knowledge_write", "structure_edit"],
            // Reviewer must not produce draft content or apply knowledge
            Self::Review => &["draft_write", "structure_edit", "knowledge_write"],
            // Knowledge manager proposes only; must not write drafts
            Self::Knowledge => &["draft_write", "structure_edit"],
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Orchestrator => "orchestrator",
            Self::Context => "context",
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Knowledge => "knowledge",
        }
    }
}

/// Render the role-specific prompt text.
pub fn render_role(role: &PromptRole) -> String {
    match role {
        PromptRole::Orchestrator => ORCHESTRATOR_ROLE.to_string(),
        PromptRole::Context => CONTEXT_ROLE.to_string(),
        PromptRole::Draft => DRAFT_ROLE.to_string(),
        PromptRole::Review => REVIEW_ROLE.to_string(),
        PromptRole::Knowledge => KNOWLEDGE_ROLE.to_string(),
    }
}

/// Check whether the assembled prompt text leaks forbidden tool names for this role.
/// Returns a list of violation messages (empty = clean).
pub fn check_forbidden_violations(role: &PromptRole, prompt_text: &str) -> Vec<String> {
    let mut violations = Vec::new();
    // Only check the role segment, not the whole assembled text,
    // to avoid flagging the core tool reference table.
    // We check the role's own rendered text here.
    let role_text = render_role(role);
    let _ = prompt_text; // reserved for future cross-layer validation

    for line in role_text.lines() {
        let lower = line.to_lowercase();

        // Ignore prohibitions like "Do NOT use ..." — these are correct and must not trigger.
        if lower.contains("do not")
            || lower.contains("don't")
            || lower.contains("dont")
            || lower.contains("never")
            || lower.contains("must not")
        {
            continue;
        }

        let tokens: Vec<&str> = lower
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .filter(|t| !t.is_empty())
            .collect();

        // Only treat affirmative "use/call" as instructions.
        let has_affirmative_verb = tokens.iter().any(|t| *t == "use" || *t == "call");
        if !has_affirmative_verb {
            continue;
        }

        for tool in role.forbidden_tools() {
            if tokens.iter().any(|t| *t == *tool) {
                violations.push(format!(
                    "role '{}' prompt instructs use of forbidden tool '{}'",
                    role.as_str(),
                    tool
                ));
            }
        }
    }
    violations
}

const ORCHESTRATOR_ROLE: &str = r#"## Role: orchestrator
- You decompose complex user requests into sub-tasks and coordinate their execution.
- Use todowrite to track and expose progress at milestone boundaries.
- Read context (workspace_map, context_read, context_search) before planning.
- Do NOT write chapter drafts directly. Do NOT apply knowledge changes directly.
- Coordinate the next valid tool calls yourself. If multi-worker execution is needed, it must go through the mission runtime, not this chat turn."#;

const CONTEXT_ROLE: &str = r#"## Role: context
- You are a read-only context specialist.
- Use workspace_map, context_read, context_search, knowledge_read to gather information.
- Synthesize and report findings; do NOT modify any content.
- Do NOT use draft_write, structure_edit, or knowledge_write."#;

const DRAFT_ROLE: &str = r#"## Role: draft
- You write and rewrite chapter text using draft_write.
- Read the target chapter and relevant context before writing.
- Follow Active Rules (words, style, POV, forbidden patterns) precisely.
- Do NOT modify project structure or propose knowledge changes during a draft pass.
- After writing, call review_check if instructed; do not self-apply knowledge updates."#;

const REVIEW_ROLE: &str = r#"## Role: review
- You review chapter content for quality, consistency, and rule compliance.
- Use review_check to run structured checks; use context_read/knowledge_read for reference.
- Report findings clearly: pass / warn / block with specific evidence.
- Do NOT produce draft content. Do NOT edit chapters. Do NOT apply knowledge changes.
- If a block is found, describe what must be fixed; the fix is performed by the draft role."#;

const KNOWLEDGE_ROLE: &str = r#"## Role: knowledge
- You manage the project knowledge base via knowledge_write with op=propose.
- Always propose changes (op=propose); never apply directly without explicit user confirmation.
- Use knowledge_read to verify existing facts before proposing updates.
- Do NOT write chapter drafts. Do NOT edit project structure.
- Present proposed changes clearly so the user can confirm or reject."#;
