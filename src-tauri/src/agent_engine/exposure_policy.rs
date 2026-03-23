use serde::{Deserialize, Serialize};

use crate::agent_tools::definition::ToolCapability;

use super::types::{AgentMode, ApprovalMode, ClarificationMode};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionSource {
    UserInteractive,
    Delegate,
    WorkflowJob,
    ReviewGate,
}

impl Default for SessionSource {
    fn default() -> Self {
        Self::UserInteractive
    }
}

impl SessionSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UserInteractive => "user_interactive",
            Self::Delegate => "delegate",
            Self::WorkflowJob => "workflow_job",
            Self::ReviewGate => "review_gate",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityPreset {
    MainInteractive,
    MainPlanning,
    HeadlessWriter,
    ReadOnlyReviewer,
    SummaryOnly,
}

impl Default for CapabilityPreset {
    fn default() -> Self {
        Self::MainInteractive
    }
}

impl CapabilityPreset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MainInteractive => "main_interactive",
            Self::MainPlanning => "main_planning",
            Self::HeadlessWriter => "headless_writer",
            Self::ReadOnlyReviewer => "read_only_reviewer",
            Self::SummaryOnly => "summary_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CapabilityPolicy {
    pub preset: CapabilityPreset,
    pub allow_delegate: bool,
    pub allow_skill_activation: bool,
    pub hidden_tools: Vec<String>,
    pub forced_tools: Vec<String>,
}

impl CapabilityPolicy {
    pub fn new(preset: CapabilityPreset) -> Self {
        let mut policy = Self {
            preset,
            allow_delegate: false,
            allow_skill_activation: false,
            hidden_tools: Vec::new(),
            forced_tools: Vec::new(),
        };

        match preset {
            CapabilityPreset::MainInteractive | CapabilityPreset::MainPlanning => {
                policy.allow_delegate = true;
            }
            CapabilityPreset::HeadlessWriter
            | CapabilityPreset::ReadOnlyReviewer
            | CapabilityPreset::SummaryOnly => {}
        }

        policy
    }

    pub fn default_for_mode(mode: AgentMode, clarification_mode: ClarificationMode) -> Self {
        match (mode, clarification_mode) {
            (AgentMode::Planning, _) => Self::new(CapabilityPreset::MainPlanning),
            (AgentMode::Writing, ClarificationMode::HeadlessDefer) => {
                Self::new(CapabilityPreset::HeadlessWriter)
            }
            (AgentMode::Writing, ClarificationMode::Interactive) => {
                Self::new(CapabilityPreset::MainInteractive)
            }
        }
    }

    pub fn normalized(mut self) -> Self {
        self.hidden_tools = normalize_tool_names(self.hidden_tools);
        self.forced_tools = normalize_tool_names(self.forced_tools);
        self
    }

    pub fn hides_tool(&self, tool_name: &str) -> bool {
        self.hidden_tools.iter().any(|tool| tool == tool_name)
    }

    pub fn forces_tool(&self, tool_name: &str) -> bool {
        self.forced_tools.iter().any(|tool| tool == tool_name)
    }

    pub fn allows_capability(&self, capability: ToolCapability) -> bool {
        match capability {
            ToolCapability::SkillActivation => self.allow_skill_activation,
            ToolCapability::Delegate => self.allow_delegate,
            other => preset_capabilities(self.preset).contains(&other),
        }
    }

    pub fn capability_denial_reason(&self, capabilities: &[ToolCapability]) -> Option<String> {
        if capabilities
            .iter()
            .any(|capability| matches!(capability, ToolCapability::SkillActivation))
            && !self.allow_skill_activation
        {
            return Some("capability_policy:skill_disabled".to_string());
        }

        if capabilities
            .iter()
            .any(|capability| matches!(capability, ToolCapability::Delegate))
            && !self.allow_delegate
        {
            return Some("capability_policy:delegate_disabled".to_string());
        }

        if capabilities
            .iter()
            .all(|capability| self.allows_capability(*capability))
        {
            None
        } else {
            Some(format!("capability_preset:{}", self.preset.as_str()))
        }
    }
}

impl Default for CapabilityPolicy {
    fn default() -> Self {
        Self::new(CapabilityPreset::MainInteractive)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ExposureContext {
    pub mode: AgentMode,
    pub approval_mode: ApprovalMode,
    pub clarification_mode: ClarificationMode,
    pub session_source: SessionSource,
    pub delegate_depth: u8,
    pub semantic_retrieval_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_chapter_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_profile: Option<String>,
    pub capability_policy: CapabilityPolicy,
}

impl ExposureContext {
    pub fn new(
        mode: AgentMode,
        approval_mode: ApprovalMode,
        clarification_mode: ClarificationMode,
        session_source: SessionSource,
        delegate_depth: u8,
        semantic_retrieval_enabled: bool,
        active_chapter_path: Option<String>,
        active_profile: Option<String>,
        capability_policy: CapabilityPolicy,
    ) -> Self {
        Self {
            mode,
            approval_mode,
            clarification_mode,
            session_source,
            delegate_depth,
            semantic_retrieval_enabled,
            active_chapter_path: normalize_optional(active_chapter_path),
            active_profile: normalize_optional(active_profile),
            capability_policy: capability_policy.normalized(),
        }
    }

    pub fn default_for_mode(
        mode: AgentMode,
        clarification_mode: ClarificationMode,
        semantic_retrieval_enabled: bool,
    ) -> Self {
        let capability_policy = CapabilityPolicy::default_for_mode(mode, clarification_mode);
        Self::new(
            mode,
            ApprovalMode::ConfirmWrites,
            clarification_mode,
            SessionSource::UserInteractive,
            0,
            semantic_retrieval_enabled,
            None,
            None,
            capability_policy,
        )
    }

    pub fn policy_summary(&self) -> ExposurePolicySummary {
        ExposurePolicySummary {
            mode: self.mode,
            approval_mode: self.approval_mode,
            clarification_mode: self.clarification_mode,
            session_source: self.session_source,
            delegate_depth: self.delegate_depth,
            capability_preset: self.capability_policy.preset,
            allow_delegate: self.capability_policy.allow_delegate,
            allow_skill_activation: self.capability_policy.allow_skill_activation,
            semantic_retrieval_enabled: self.semantic_retrieval_enabled,
            hidden_tool_overrides: self.capability_policy.hidden_tools.clone(),
            forced_tool_overrides: self.capability_policy.forced_tools.clone(),
            active_profile: self.active_profile.clone(),
            active_chapter_present: self
                .active_chapter_path
                .as_ref()
                .map(|path| !path.trim().is_empty())
                .unwrap_or(false),
        }
    }
}

impl Default for ExposureContext {
    fn default() -> Self {
        Self::default_for_mode(AgentMode::Writing, ClarificationMode::Interactive, false)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExposurePolicySummary {
    pub mode: AgentMode,
    pub approval_mode: ApprovalMode,
    pub clarification_mode: ClarificationMode,
    pub session_source: SessionSource,
    pub delegate_depth: u8,
    pub capability_preset: CapabilityPreset,
    pub allow_delegate: bool,
    pub allow_skill_activation: bool,
    pub semantic_retrieval_enabled: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hidden_tool_overrides: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forced_tool_overrides: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_profile: Option<String>,
    pub active_chapter_present: bool,
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

fn normalize_tool_names(raw: Vec<String>) -> Vec<String> {
    let mut normalized = raw
        .into_iter()
        .map(|tool| tool.trim().to_ascii_lowercase())
        .filter(|tool| !tool.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

const MAIN_INTERACTIVE_CAPABILITIES: &[ToolCapability] = &[
    ToolCapability::WorkspaceRead,
    ToolCapability::ContextRead,
    ToolCapability::Search,
    ToolCapability::KnowledgeRead,
    ToolCapability::KnowledgeWrite,
    ToolCapability::DraftWrite,
    ToolCapability::StructureWrite,
    ToolCapability::Review,
    ToolCapability::Todo,
    ToolCapability::AskUser,
];

const MAIN_PLANNING_CAPABILITIES: &[ToolCapability] = &[
    ToolCapability::WorkspaceRead,
    ToolCapability::ContextRead,
    ToolCapability::Search,
    ToolCapability::KnowledgeRead,
    ToolCapability::Review,
    ToolCapability::Todo,
    ToolCapability::AskUser,
];

const HEADLESS_WRITER_CAPABILITIES: &[ToolCapability] = &[
    ToolCapability::WorkspaceRead,
    ToolCapability::ContextRead,
    ToolCapability::Search,
    ToolCapability::KnowledgeRead,
    ToolCapability::KnowledgeWrite,
    ToolCapability::DraftWrite,
    ToolCapability::StructureWrite,
    ToolCapability::Review,
    ToolCapability::Todo,
];

const READ_ONLY_REVIEWER_CAPABILITIES: &[ToolCapability] = &[
    ToolCapability::WorkspaceRead,
    ToolCapability::ContextRead,
    ToolCapability::Search,
    ToolCapability::KnowledgeRead,
    ToolCapability::Review,
    ToolCapability::Todo,
];

const SUMMARY_ONLY_CAPABILITIES: &[ToolCapability] = &[
    ToolCapability::WorkspaceRead,
    ToolCapability::ContextRead,
    ToolCapability::Search,
    ToolCapability::KnowledgeRead,
    ToolCapability::Review,
];

fn preset_capabilities(preset: CapabilityPreset) -> &'static [ToolCapability] {
    match preset {
        CapabilityPreset::MainInteractive => MAIN_INTERACTIVE_CAPABILITIES,
        CapabilityPreset::MainPlanning => MAIN_PLANNING_CAPABILITIES,
        CapabilityPreset::HeadlessWriter => HEADLESS_WRITER_CAPABILITIES,
        CapabilityPreset::ReadOnlyReviewer => READ_ONLY_REVIEWER_CAPABILITIES,
        CapabilityPreset::SummaryOnly => SUMMARY_ONLY_CAPABILITIES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_tracks_mode_and_clarification() {
        assert_eq!(
            CapabilityPolicy::default_for_mode(AgentMode::Writing, ClarificationMode::Interactive)
                .preset,
            CapabilityPreset::MainInteractive
        );
        assert_eq!(
            CapabilityPolicy::default_for_mode(AgentMode::Planning, ClarificationMode::Interactive)
                .preset,
            CapabilityPreset::MainPlanning
        );
        assert_eq!(
            CapabilityPolicy::default_for_mode(
                AgentMode::Writing,
                ClarificationMode::HeadlessDefer
            )
            .preset,
            CapabilityPreset::HeadlessWriter
        );
    }

    #[test]
    fn normalized_policy_deduplicates_tool_overrides() {
        let policy = CapabilityPolicy {
            preset: CapabilityPreset::MainInteractive,
            allow_delegate: true,
            allow_skill_activation: false,
            hidden_tools: vec![
                " askuser ".to_string(),
                "ASKUSER".to_string(),
                "skill".to_string(),
            ],
            forced_tools: vec!["skill".to_string(), " skill ".to_string()],
        }
        .normalized();

        assert_eq!(
            policy.hidden_tools,
            vec!["askuser".to_string(), "skill".to_string()]
        );
        assert_eq!(policy.forced_tools, vec!["skill".to_string()]);
    }

    #[test]
    fn capability_denial_reason_prefers_skill_and_delegate_flags() {
        let policy = CapabilityPolicy::new(CapabilityPreset::MainInteractive);

        assert_eq!(
            policy.capability_denial_reason(&[ToolCapability::SkillActivation]),
            Some("capability_policy:skill_disabled".to_string())
        );
        assert_eq!(
            policy.capability_denial_reason(&[ToolCapability::Delegate]),
            None
        );
    }

    #[test]
    fn exposure_policy_summary_reports_overrides() {
        let exposure = ExposureContext::new(
            AgentMode::Writing,
            ApprovalMode::ConfirmWrites,
            ClarificationMode::Interactive,
            SessionSource::UserInteractive,
            0,
            true,
            Some("manuscripts/vol_1/ch_1.json".to_string()),
            Some("writer".to_string()),
            CapabilityPolicy {
                preset: CapabilityPreset::MainInteractive,
                allow_delegate: true,
                allow_skill_activation: false,
                hidden_tools: vec!["skill".to_string()],
                forced_tools: vec!["review_check".to_string()],
            },
        );

        let summary = exposure.policy_summary();
        assert_eq!(summary.capability_preset, CapabilityPreset::MainInteractive);
        assert!(summary.semantic_retrieval_enabled);
        assert!(summary.active_chapter_present);
        assert_eq!(summary.active_profile.as_deref(), Some("writer"));
        assert_eq!(summary.hidden_tool_overrides, vec!["skill".to_string()]);
        assert_eq!(
            summary.forced_tool_overrides,
            vec!["review_check".to_string()]
        );
    }
}
