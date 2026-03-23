use super::types::ToolCallInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerType {
    Context,
    Draft,
    Review,
    Knowledge,
    Orchestrator,
    Other,
}

impl WorkerType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Knowledge => "knowledge",
            Self::Orchestrator => "orchestrator",
            Self::Other => "other",
        }
    }

    fn priority(self) -> u8 {
        match self {
            // Priority is used when selecting a single worker_type for a parallel batch.
            // Higher = more important to surface as the "current phase".
            Self::Knowledge => 60,
            Self::Review => 50,
            Self::Draft => 40,
            Self::Context => 30,
            Self::Orchestrator => 20,
            Self::Other => 10,
        }
    }
}

pub fn worker_type_for_tool_name(tool_name: &str) -> WorkerType {
    let name = tool_name.trim().to_ascii_lowercase();

    // Session control / orchestration
    if name == "todowrite" || name == "skill" || name == "askuser" {
        return WorkerType::Orchestrator;
    }

    // Explicit role-prefixed tools
    if name.starts_with("context_") {
        return WorkerType::Context;
    }
    if name.starts_with("draft_") {
        return WorkerType::Draft;
    }
    if name == "review_check" || name.starts_with("review_") {
        return WorkerType::Review;
    }
    if name == "knowledge_write" {
        return WorkerType::Knowledge;
    }

    // Knowledge reads are context gathering (not "knowledge writeback").
    if name == "knowledge_read" || name.starts_with("knowledge_") {
        return WorkerType::Context;
    }

    // Common context-gathering tools (parallel-safe)
    if matches!(name.as_str(), "workspace_map") {
        return WorkerType::Context;
    }

    // Structure edits are a separate domain in the current toolset; map to "other" (V1).
    if name == "structure_edit" || name.starts_with("structure_") {
        return WorkerType::Other;
    }

    WorkerType::Other
}

pub fn worker_type_for_tool_batch(tool_calls: &[ToolCallInfo]) -> WorkerType {
    tool_calls
        .iter()
        .map(|tc| worker_type_for_tool_name(&tc.tool_name))
        .max_by_key(|worker_type| worker_type.priority())
        .unwrap_or(WorkerType::Other)
}

#[derive(Debug, Clone)]
pub struct WorkerRefs {
    pub scope_ref: Option<String>,
    pub target_ref: Option<String>,
}

pub fn extract_worker_refs(tc: &ToolCallInfo) -> WorkerRefs {
    let args = &tc.args;
    let scope_ref = args
        .get("scope_ref")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());

    let mut target_ref = args
        .get("target_ref")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());

    if target_ref.is_none() {
        target_ref = args
            .get("target_refs")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());
    }

    // knowledge_write uses changes[].target_ref
    if target_ref.is_none() && tc.tool_name.trim().eq_ignore_ascii_case("knowledge_write") {
        target_ref = args
            .get("changes")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.as_object())
            .and_then(|map| map.get("target_ref"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());
    }

    // knowledge_read uses item_ref
    if target_ref.is_none() && tc.tool_name.trim().eq_ignore_ascii_case("knowledge_read") {
        target_ref = args
            .get("item_ref")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());
    }

    WorkerRefs {
        scope_ref,
        target_ref,
    }
}
