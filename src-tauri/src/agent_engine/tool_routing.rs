use serde::Serialize;
use serde_json::json;

use crate::agent_engine::messages::{AgentMessage, ConversationState, Role};
use crate::agent_engine::tool_schemas::{
    build_filtered_tool_schema_bundle, build_tool_schema_bundle, BuiltToolSchemas,
};
use crate::agent_engine::types::{AgentMode, LoopConfig};
use crate::agent_tools::registry::ToolSchemaSkipDiagnostic;

const TOOL_PACKAGE_ROLLOUT_MODE_ENV: &str = "MAGIC_TOOL_PACKAGE_ROLLOUT_MODE";
const TOOL_PACKAGE_CANARY_PERCENT_ENV: &str = "MAGIC_TOOL_PACKAGE_CANARY_PERCENT";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolPackageRolloutMode {
    On,
    Canary,
    Off,
}

impl ToolPackageRolloutMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Canary => "canary",
            Self::Off => "off",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToolPackageRolloutConfig {
    mode: ToolPackageRolloutMode,
    canary_percent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToolPackageRolloutDecision {
    dynamic_exposure_enabled: bool,
    mode: ToolPackageRolloutMode,
    in_canary: bool,
    canary_percent: Option<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolPackageName {
    LightChat,
    Writing,
    StructureOps,
    Research,
    CustomWhitelist,
    LegacyFull,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct ToolExposureTelemetry {
    pub tool_package: ToolPackageName,
    pub route_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_from: Option<ToolPackageName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub rollout_mode: String,
    pub rollout_in_canary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canary_percent: Option<u8>,
    pub exposed_tools: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped_tools: Vec<ToolSchemaSkipDiagnostic>,
}

impl ToolExposureTelemetry {
    pub fn to_payload(&self) -> serde_json::Value {
        json!({
            "tool_package": self.tool_package,
            "route_reason": self.route_reason,
            "fallback_from": self.fallback_from,
            "fallback_reason": self.fallback_reason,
            "rollout_mode": self.rollout_mode,
            "rollout_in_canary": self.rollout_in_canary,
            "canary_percent": self.canary_percent,
            "exposed_tools": self.exposed_tools,
            "skipped_tools": self.skipped_tools,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedToolExposure {
    pub bundle: BuiltToolSchemas,
    pub telemetry: ToolExposureTelemetry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolPackageDecision {
    package: ToolPackageName,
    route_reason: String,
    fallback_from: Option<ToolPackageName>,
    fallback_reason: Option<String>,
    include_optional_todowrite: bool,
}

const LIGHT_CHAT_TOOLS: &[&str] = &["read", "grep", "askuser"];
const WRITING_TOOLS: &[&str] = &[
    "read",
    "edit",
    "grep",
    "review_check",
    "outline",
    "character_sheet",
    "search_knowledge",
    "askuser",
    "todowrite",
];
const STRUCTURE_OPS_TOOLS: &[&str] = &[
    "ls",
    "read",
    "create",
    "delete",
    "move",
    "askuser",
    "todowrite",
];
const RESEARCH_TOOLS: &[&str] = &[
    "read",
    "grep",
    "ls",
    "review_check",
    "outline",
    "character_sheet",
    "search_knowledge",
    "askuser",
];
const LEGACY_FULL_TOOLS: &[&str] = &[
    "read",
    "edit",
    "create",
    "delete",
    "move",
    "ls",
    "grep",
    "review_check",
    "outline",
    "character_sheet",
    "search_knowledge",
    "askuser",
    "todowrite",
    "skill",
];

pub fn package_tool_whitelist(
    package: ToolPackageName,
    include_optional_todowrite: bool,
    custom_whitelist: Option<&[String]>,
) -> Vec<String> {
    match package {
        ToolPackageName::LightChat => {
            let mut tools = LIGHT_CHAT_TOOLS
                .iter()
                .map(|tool| tool.to_string())
                .collect::<Vec<_>>();
            if include_optional_todowrite {
                tools.push("todowrite".to_string());
            }
            tools
        }
        ToolPackageName::Writing => WRITING_TOOLS.iter().map(|tool| tool.to_string()).collect(),
        ToolPackageName::StructureOps => STRUCTURE_OPS_TOOLS
            .iter()
            .map(|tool| tool.to_string())
            .collect(),
        ToolPackageName::Research => RESEARCH_TOOLS.iter().map(|tool| tool.to_string()).collect(),
        ToolPackageName::LegacyFull => LEGACY_FULL_TOOLS
            .iter()
            .map(|tool| tool.to_string())
            .collect(),
        ToolPackageName::CustomWhitelist => custom_whitelist
            .unwrap_or(&[])
            .iter()
            .map(|tool| tool.trim().to_string())
            .filter(|tool| !tool.is_empty())
            .collect(),
    }
}

pub(crate) fn resolve_turn_tool_exposure(
    state: &ConversationState,
    config: &LoopConfig,
    active_chapter_path: Option<&str>,
    worker_tool_whitelist: Option<&[String]>,
    semantic_retrieval_enabled: bool,
) -> ResolvedToolExposure {
    let rollout =
        resolve_rollout_decision_for_session(&state.session_id, resolve_rollout_config_from_env());
    resolve_turn_tool_exposure_with_rollout(
        state,
        config,
        active_chapter_path,
        worker_tool_whitelist,
        semantic_retrieval_enabled,
        rollout,
    )
}

fn resolve_turn_tool_exposure_with_rollout(
    state: &ConversationState,
    config: &LoopConfig,
    active_chapter_path: Option<&str>,
    worker_tool_whitelist: Option<&[String]>,
    semantic_retrieval_enabled: bool,
    rollout: ToolPackageRolloutDecision,
) -> ResolvedToolExposure {
    if let Some(whitelist) = worker_tool_whitelist {
        let bundle = build_filtered_tool_schema_bundle(
            whitelist,
            config.clarification_mode,
            semantic_retrieval_enabled,
            config.capability_mode,
        );
        let telemetry = ToolExposureTelemetry {
            tool_package: ToolPackageName::CustomWhitelist,
            route_reason: "worker_tool_whitelist".to_string(),
            fallback_from: None,
            fallback_reason: None,
            rollout_mode: "worker_override".to_string(),
            rollout_in_canary: rollout.in_canary,
            canary_percent: rollout.canary_percent,
            exposed_tools: bundle.exposed_tools.clone(),
            skipped_tools: bundle.skipped_tools.clone(),
        };

        return ResolvedToolExposure { bundle, telemetry };
    }

    if !rollout.dynamic_exposure_enabled {
        let bundle = build_tool_schema_bundle(
            config.clarification_mode,
            semantic_retrieval_enabled,
            config.capability_mode,
        );
        let telemetry = ToolExposureTelemetry {
            tool_package: ToolPackageName::LegacyFull,
            route_reason: format!("rollout_legacy_full.{}", rollout.mode.as_str()),
            fallback_from: None,
            fallback_reason: None,
            rollout_mode: rollout.mode.as_str().to_string(),
            rollout_in_canary: rollout.in_canary,
            canary_percent: rollout.canary_percent,
            exposed_tools: bundle.exposed_tools.clone(),
            skipped_tools: bundle.skipped_tools.clone(),
        };

        return ResolvedToolExposure { bundle, telemetry };
    }

    let user_text = latest_user_text(state);
    let decision = resolve_package_decision(
        &user_text,
        config.capability_mode,
        active_chapter_path,
        recent_structure_operation(state),
    );
    let whitelist =
        package_tool_whitelist(decision.package, decision.include_optional_todowrite, None);
    let bundle = build_filtered_tool_schema_bundle(
        &whitelist,
        config.clarification_mode,
        semantic_retrieval_enabled,
        config.capability_mode,
    );
    let telemetry = ToolExposureTelemetry {
        tool_package: decision.package,
        route_reason: decision.route_reason,
        fallback_from: decision.fallback_from,
        fallback_reason: decision.fallback_reason,
        rollout_mode: rollout.mode.as_str().to_string(),
        rollout_in_canary: rollout.in_canary,
        canary_percent: rollout.canary_percent,
        exposed_tools: bundle.exposed_tools.clone(),
        skipped_tools: bundle.skipped_tools.clone(),
    };

    ResolvedToolExposure { bundle, telemetry }
}

fn resolve_rollout_config_from_env() -> ToolPackageRolloutConfig {
    let mode = std::env::var(TOOL_PACKAGE_ROLLOUT_MODE_ENV)
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .map(|raw| match raw.as_str() {
            "off" | "disabled" | "false" | "0" | "legacy" => ToolPackageRolloutMode::Off,
            "canary" => ToolPackageRolloutMode::Canary,
            _ => ToolPackageRolloutMode::On,
        })
        .unwrap_or(ToolPackageRolloutMode::On);

    let canary_percent = std::env::var(TOOL_PACKAGE_CANARY_PERCENT_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u8>().ok())
        .map(|value| value.min(100))
        .unwrap_or(10);

    ToolPackageRolloutConfig {
        mode,
        canary_percent,
    }
}

fn resolve_rollout_decision_for_session(
    session_id: &str,
    config: ToolPackageRolloutConfig,
) -> ToolPackageRolloutDecision {
    match config.mode {
        ToolPackageRolloutMode::On => ToolPackageRolloutDecision {
            dynamic_exposure_enabled: true,
            mode: config.mode,
            in_canary: true,
            canary_percent: None,
        },
        ToolPackageRolloutMode::Off => ToolPackageRolloutDecision {
            dynamic_exposure_enabled: false,
            mode: config.mode,
            in_canary: false,
            canary_percent: None,
        },
        ToolPackageRolloutMode::Canary => {
            let in_canary = is_session_in_canary(session_id, config.canary_percent);
            ToolPackageRolloutDecision {
                dynamic_exposure_enabled: in_canary,
                mode: config.mode,
                in_canary,
                canary_percent: Some(config.canary_percent),
            }
        }
    }
}

fn is_session_in_canary(session_id: &str, percent: u8) -> bool {
    if percent == 0 {
        return false;
    }
    if percent >= 100 {
        return true;
    }
    canary_bucket(session_id) < percent
}

fn canary_bucket(session_id: &str) -> u8 {
    const OFFSET_BASIS: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;

    let mut hash = OFFSET_BASIS;
    for byte in session_id.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    (hash % 100) as u8
}

fn resolve_package_decision(
    user_text: &str,
    capability_mode: AgentMode,
    active_chapter_path: Option<&str>,
    recent_structure_operation: bool,
) -> ToolPackageDecision {
    let normalized = normalize_text(user_text);
    let has_active_chapter = active_chapter_path
        .map(|path| !path.trim().is_empty())
        .unwrap_or(false);
    let strong_structure = has_strong_structure_signal(&normalized);
    let strong_writing = has_strong_writing_signal(&normalized);
    let strong_research = has_strong_research_signal(&normalized);
    let multi_step = looks_multi_step_task(user_text);

    let mut decision = if strong_structure && !has_active_chapter {
        ToolPackageDecision {
            package: ToolPackageName::StructureOps,
            route_reason: route_reason(capability_mode, "explicit_structure_request"),
            fallback_from: None,
            fallback_reason: None,
            include_optional_todowrite: false,
        }
    } else if is_light_chat_request(&normalized) {
        ToolPackageDecision {
            package: ToolPackageName::LightChat,
            route_reason: route_reason(capability_mode, "light_chat_request"),
            fallback_from: None,
            fallback_reason: None,
            include_optional_todowrite: multi_step,
        }
    } else if has_active_chapter {
        ToolPackageDecision {
            package: ToolPackageName::Writing,
            route_reason: route_reason(capability_mode, "active_chapter_default"),
            fallback_from: None,
            fallback_reason: None,
            include_optional_todowrite: false,
        }
    } else if recent_structure_operation {
        ToolPackageDecision {
            package: ToolPackageName::StructureOps,
            route_reason: route_reason(capability_mode, "recent_structure_operation"),
            fallback_from: None,
            fallback_reason: None,
            include_optional_todowrite: false,
        }
    } else {
        ToolPackageDecision {
            package: ToolPackageName::LightChat,
            route_reason: route_reason(capability_mode, "conservative_default"),
            fallback_from: None,
            fallback_reason: None,
            include_optional_todowrite: multi_step,
        }
    };

    if matches!(decision.package, ToolPackageName::LightChat) {
        if strong_research {
            decision = ToolPackageDecision {
                package: ToolPackageName::Research,
                route_reason: route_reason(capability_mode, "research_signal"),
                fallback_from: Some(ToolPackageName::LightChat),
                fallback_reason: Some("light_chat_to_research".to_string()),
                include_optional_todowrite: false,
            };
        } else if strong_writing
            || (has_active_chapter && references_active_chapter_context(&normalized))
        {
            decision = ToolPackageDecision {
                package: ToolPackageName::Writing,
                route_reason: route_reason(capability_mode, "writing_signal"),
                fallback_from: Some(ToolPackageName::LightChat),
                fallback_reason: Some("light_chat_to_writing".to_string()),
                include_optional_todowrite: false,
            };
        }
    }

    if matches!(decision.package, ToolPackageName::Writing) && strong_structure {
        decision = ToolPackageDecision {
            package: ToolPackageName::StructureOps,
            route_reason: route_reason(capability_mode, "structure_signal"),
            fallback_from: Some(ToolPackageName::Writing),
            fallback_reason: Some("writing_to_structure_ops".to_string()),
            include_optional_todowrite: false,
        };
    }

    decision
}

fn route_reason(capability_mode: AgentMode, base_reason: &str) -> String {
    match capability_mode {
        AgentMode::Writing => base_reason.to_string(),
        AgentMode::Planning => format!("{base_reason}.planning_mode"),
    }
}

fn latest_user_text(state: &ConversationState) -> String {
    state
        .messages
        .iter()
        .rev()
        .find(|message| matches!(message.role, Role::User))
        .map(AgentMessage::text_content)
        .unwrap_or_default()
}

fn recent_structure_operation(state: &ConversationState) -> bool {
    state
        .messages
        .iter()
        .rev()
        .take(12)
        .any(message_has_structure_operation)
}

fn message_has_structure_operation(message: &AgentMessage) -> bool {
    message.blocks.iter().any(|block| match block {
        crate::agent_engine::messages::ContentBlock::ToolCall { name, .. } => {
            matches!(name.as_str(), "create" | "delete" | "move")
        }
        crate::agent_engine::messages::ContentBlock::ToolResult { tool_name, .. } => tool_name
            .as_deref()
            .map(|name| matches!(name, "create" | "delete" | "move"))
            .unwrap_or(false),
        _ => false,
    })
}

fn normalize_text(input: &str) -> String {
    input.trim().to_lowercase()
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn is_light_chat_request(text: &str) -> bool {
    if text.trim().is_empty() {
        return true;
    }

    let greeting = contains_any(
        text,
        &[
            "hello",
            "hi",
            "hey",
            "thanks",
            "thank you",
            "你好",
            "嗨",
            "在吗",
            "谢谢",
            "早上好",
            "下午好",
            "晚上好",
        ],
    );

    greeting && text.chars().count() <= 48
}

fn has_strong_structure_signal(text: &str) -> bool {
    contains_any(
        text,
        &[
            "create chapter",
            "new chapter",
            "create volume",
            "new volume",
            "delete chapter",
            "delete volume",
            "move chapter",
            "reorder chapter",
            "list chapters",
            "project structure",
            "目录结构",
            "项目结构",
            "新建章节",
            "创建章节",
            "创建新章节",
            "新建卷",
            "创建卷",
            "删除章节",
            "删除卷",
            "移动章节",
            "重排章节",
            "重排卷章",
            "章节目录",
        ],
    )
}

fn has_strong_writing_signal(text: &str) -> bool {
    contains_any(
        text,
        &[
            "rewrite",
            "revise",
            "edit chapter",
            "continue writing",
            "continue this chapter",
            "polish",
            "draft",
            "scene",
            "paragraph",
            "prose",
            "续写",
            "润色",
            "改写",
            "重写",
            "修订",
            "修改正文",
            "修改章节",
            "段落",
            "正文",
            "场景",
        ],
    )
}

fn has_strong_research_signal(text: &str) -> bool {
    contains_any(
        text,
        &[
            "outline",
            "character",
            "knowledge base",
            "setting",
            "worldbuilding",
            "consistency",
            "review",
            "research",
            "search notes",
            "角色",
            "设定",
            "世界观",
            "大纲",
            "梳理",
            "检索",
            "搜索",
            "一致性",
            "查设定",
            "查角色",
        ],
    )
}

fn references_active_chapter_context(text: &str) -> bool {
    contains_any(
        text,
        &[
            "chapter",
            "scene",
            "paragraph",
            "selection",
            "current draft",
            "当前章节",
            "当前段落",
            "选中内容",
            "这一段",
        ],
    )
}

fn looks_multi_step_task(text: &str) -> bool {
    contains_any(
        &text.to_lowercase(),
        &[
            "first ",
            " then ",
            " after that",
            "先",
            "然后",
            "再",
            "步骤",
            "1.",
            "2.",
            "- ",
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::messages::{AgentMessage, ContentBlock};
    use crate::agent_engine::types::{ApprovalMode, ClarificationMode};

    fn config(mode: AgentMode) -> LoopConfig {
        LoopConfig {
            capability_mode: mode,
            approval_mode: ApprovalMode::ConfirmWrites,
            clarification_mode: ClarificationMode::Interactive,
            ..LoopConfig::default()
        }
    }

    #[test]
    fn package_whitelists_are_scoped_and_stable() {
        let light = package_tool_whitelist(ToolPackageName::LightChat, false, None);
        let writing = package_tool_whitelist(ToolPackageName::Writing, false, None);
        let structure = package_tool_whitelist(ToolPackageName::StructureOps, false, None);
        let research = package_tool_whitelist(ToolPackageName::Research, false, None);

        assert_eq!(light, vec!["read", "grep", "askuser"]);
        assert!(writing.contains(&"edit".to_string()));
        assert!(writing.contains(&"review_check".to_string()));
        assert!(structure.contains(&"move".to_string()));
        assert!(!structure.contains(&"review_check".to_string()));
        assert!(research.contains(&"search_knowledge".to_string()));
        assert!(research.contains(&"review_check".to_string()));
        assert!(
            light.len() < 13 && writing.len() < 13 && structure.len() < 13 && research.len() < 13
        );
    }

    #[test]
    fn light_chat_optionally_adds_todowrite_for_multistep_requests() {
        let decision = resolve_package_decision(
            "First inspect the request, then explain what you will do.",
            AgentMode::Writing,
            None,
            false,
        );

        assert_eq!(decision.package, ToolPackageName::LightChat);
        assert!(decision.include_optional_todowrite);
    }

    #[test]
    fn rewrite_request_falls_back_from_light_chat_to_writing() {
        let decision = resolve_package_decision(
            "Please rewrite chapter 3 in a tighter voice.",
            AgentMode::Writing,
            None,
            false,
        );

        assert_eq!(decision.package, ToolPackageName::Writing);
        assert_eq!(decision.fallback_from, Some(ToolPackageName::LightChat));
        assert_eq!(
            decision.fallback_reason.as_deref(),
            Some("light_chat_to_writing")
        );
    }

    #[test]
    fn research_request_falls_back_from_light_chat_to_research() {
        let decision = resolve_package_decision(
            "Can you check the character setting consistency?",
            AgentMode::Writing,
            None,
            false,
        );

        assert_eq!(decision.package, ToolPackageName::Research);
        assert_eq!(decision.fallback_from, Some(ToolPackageName::LightChat));
        assert_eq!(
            decision.fallback_reason.as_deref(),
            Some("light_chat_to_research")
        );
    }

    #[test]
    fn active_chapter_structure_request_falls_back_to_structure_ops() {
        let decision = resolve_package_decision(
            "Create a new chapter after this one.",
            AgentMode::Writing,
            Some("manuscripts/vol_1/ch_1.json"),
            false,
        );

        assert_eq!(decision.package, ToolPackageName::StructureOps);
        assert_eq!(decision.fallback_from, Some(ToolPackageName::Writing));
        assert_eq!(
            decision.fallback_reason.as_deref(),
            Some("writing_to_structure_ops")
        );
    }

    #[test]
    fn recent_structure_operation_biases_conservative_turns_to_structure_ops() {
        let mut state = ConversationState::new("sess_route".to_string());
        state
            .messages
            .push(AgentMessage::user("move the chapter".to_string()));
        state.messages.push(AgentMessage {
            id: "msg_tool".to_string(),
            role: Role::Assistant,
            blocks: vec![ContentBlock::ToolCall {
                id: "call_1".to_string(),
                name: "move".to_string(),
                input: json!({}),
            }],
            ts: 0,
        });
        state
            .messages
            .push(AgentMessage::user("continue".to_string()));

        let resolved =
            resolve_turn_tool_exposure(&state, &config(AgentMode::Writing), None, None, false);
        assert_eq!(
            resolved.telemetry.tool_package,
            ToolPackageName::StructureOps
        );
        assert_eq!(
            resolved.telemetry.route_reason,
            "recent_structure_operation"
        );
    }

    #[test]
    fn planning_mode_reason_is_annotated() {
        let resolved = resolve_turn_tool_exposure(
            &ConversationState {
                session_id: "sess_plan".to_string(),
                messages: vec![AgentMessage::user(
                    "Please rewrite this chapter".to_string(),
                )],
                current_turn: 1,
                total_tool_calls: 0,
                last_compaction: None,
                last_usage: None,
            },
            &config(AgentMode::Planning),
            None,
            None,
            false,
        );

        assert!(resolved.telemetry.route_reason.ends_with(".planning_mode"));
        assert!(!resolved
            .telemetry
            .exposed_tools
            .contains(&"edit".to_string()));
    }

    #[test]
    fn rollout_off_uses_legacy_full_package() {
        let state = ConversationState {
            session_id: "sess_rollout_off".to_string(),
            messages: vec![AgentMessage::user("hello".to_string())],
            current_turn: 1,
            total_tool_calls: 0,
            last_compaction: None,
            last_usage: None,
        };

        let rollout = resolve_rollout_decision_for_session(
            &state.session_id,
            ToolPackageRolloutConfig {
                mode: ToolPackageRolloutMode::Off,
                canary_percent: 10,
            },
        );
        let resolved = resolve_turn_tool_exposure_with_rollout(
            &state,
            &config(AgentMode::Writing),
            None,
            None,
            false,
            rollout,
        );

        assert_eq!(resolved.telemetry.tool_package, ToolPackageName::LegacyFull);
        assert_eq!(resolved.telemetry.rollout_mode, "off");
        assert!(resolved
            .telemetry
            .exposed_tools
            .contains(&"create".to_string()));
        assert!(resolved
            .telemetry
            .exposed_tools
            .contains(&"search_knowledge".to_string()));
    }

    #[test]
    fn rollout_canary_decision_honors_percentage_bounds() {
        let always = resolve_rollout_decision_for_session(
            "sess_canary",
            ToolPackageRolloutConfig {
                mode: ToolPackageRolloutMode::Canary,
                canary_percent: 100,
            },
        );
        assert!(always.dynamic_exposure_enabled);

        let never = resolve_rollout_decision_for_session(
            "sess_canary",
            ToolPackageRolloutConfig {
                mode: ToolPackageRolloutMode::Canary,
                canary_percent: 0,
            },
        );
        assert!(!never.dynamic_exposure_enabled);
    }
}
