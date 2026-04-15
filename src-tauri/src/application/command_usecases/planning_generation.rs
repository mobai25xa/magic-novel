use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;

use crate::agent_engine::messages::AgentMessage;
use crate::application::command_usecases::inspiration::{
    CreateProjectHandoffDraft, InspirationConsensusState,
};
use crate::application::command_usecases::planning_bundle::{
    build_deterministic_planning_bundle, build_planning_bundle_from_core_docs, PlanningBundle,
    PlanningDoc, PlanningGenerationMetadata,
};
use crate::llm::accumulator::StreamAccumulator;
use crate::llm::provider::new_cancel_token;
use crate::llm::router::RetryConfig;
use crate::llm::router_factory::build_router;
use crate::llm::types::{LlmRequest, SystemBlock, ToolChoice};
use crate::models::{AppError, ErrorCode, PlanningDocId, ProjectMetadata};
use crate::services::ai_settings::{
    resolve_planning_generation_config, ResolvedPlanningGenerationConfig,
};

pub async fn generate_planning_bundle(
    project: &ProjectMetadata,
    consensus_snapshot: &InspirationConsensusState,
    create_handoff: &CreateProjectHandoffDraft,
) -> Result<PlanningBundle, AppError> {
    let config = resolve_planning_generation_config()?;
    generate_planning_bundle_with_config(project, consensus_snapshot, create_handoff, &config).await
}

pub(crate) async fn generate_planning_bundle_with_config(
    project: &ProjectMetadata,
    consensus_snapshot: &InspirationConsensusState,
    create_handoff: &CreateProjectHandoffDraft,
    config: &ResolvedPlanningGenerationConfig,
) -> Result<PlanningBundle, AppError> {
    let generation = if config.can_use_llm {
        PlanningGenerationMetadata {
            generation_source: config.source_tag.clone(),
            generation_provider: Some(config.provider_type.clone()),
            generation_model: Some(config.model.clone()),
        }
    } else {
        PlanningGenerationMetadata {
            generation_source: "deterministic_fallback".to_string(),
            generation_provider: None,
            generation_model: None,
        }
    };

    if !config.can_use_llm {
        return build_deterministic_planning_bundle(
            project,
            consensus_snapshot,
            create_handoff,
            generation,
        );
    }

    let payload =
        request_llm_planning_payload(project, consensus_snapshot, create_handoff, config).await?;
    let docs = vec![
        PlanningDoc {
            doc_id: PlanningDocId::StoryBrief,
            content: payload.story_brief.trim().to_string(),
        },
        PlanningDoc {
            doc_id: PlanningDocId::StoryBlueprint,
            content: payload.story_blueprint.trim().to_string(),
        },
        PlanningDoc {
            doc_id: PlanningDocId::NarrativeContract,
            content: payload.narrative_contract.trim().to_string(),
        },
        PlanningDoc {
            doc_id: PlanningDocId::CharacterCards,
            content: payload.character_cards.trim().to_string(),
        },
        PlanningDoc {
            doc_id: PlanningDocId::ForeshadowRegistry,
            content: payload.foreshadow_registry.trim().to_string(),
        },
        PlanningDoc {
            doc_id: PlanningDocId::ChapterPlanning,
            content: payload.chapter_planning.trim().to_string(),
        },
    ];

    build_planning_bundle_from_core_docs(project, docs, generation)
}

#[derive(Debug, Deserialize)]
struct LlmPlanningPayload {
    story_brief: String,
    story_blueprint: String,
    narrative_contract: String,
    character_cards: String,
    foreshadow_registry: String,
    chapter_planning: String,
}

async fn request_llm_planning_payload(
    project: &ProjectMetadata,
    consensus_snapshot: &InspirationConsensusState,
    create_handoff: &CreateProjectHandoffDraft,
    config: &ResolvedPlanningGenerationConfig,
) -> Result<LlmPlanningPayload, AppError> {
    let resolved = consensus_snapshot
        .resolve_for_variants()
        .map_err(|missing| AppError {
            code: ErrorCode::InvalidArgument,
            message: "missing minimum consensus for project creation".to_string(),
            details: Some(json!({
                "code": "MissingMinimumConsensus",
                "missing_fields": missing.into_iter().map(|field| field.as_str()).collect::<Vec<_>>()
            })),
            recoverable: Some(true),
        })?;

    let router = build_router(
        &config.provider_type,
        config.base_url.clone(),
        config.api_key.clone(),
        RetryConfig::worker(),
    );
    let request = LlmRequest {
        provider_name: config.provider_type.clone(),
        model: config.model.clone(),
        system: vec![SystemBlock {
            text: build_system_prompt(),
            cache_control: None,
        }],
        messages: vec![AgentMessage::user(build_user_prompt(
            project,
            &resolved,
            create_handoff,
        )?)],
        tools: Vec::new(),
        tool_choice: ToolChoice::None,
        parallel_tool_calls: false,
        temperature: 0.2,
        reasoning: None,
    };

    let (_cancel_tx, cancel_rx) = new_cancel_token();
    let mut stream = router
        .stream_chat(request, cancel_rx)
        .await
        .map_err(|error| {
            core_bundle_failure("planning llm request failed", Some(error.to_string()), None)
        })?;
    let mut accumulator = StreamAccumulator::new();
    while let Some(event) = stream.next().await {
        let event = event.map_err(|error| {
            core_bundle_failure("planning llm stream failed", Some(error.to_string()), None)
        })?;
        accumulator.apply(&event);
    }

    let output = accumulator.into_turn_output().map_err(|error| {
        core_bundle_failure(
            "planning llm output assembly failed",
            Some(error.to_string()),
            None,
        )
    })?;
    if !output.tool_calls.is_empty() {
        return Err(core_bundle_failure(
            "planning llm response must not contain tool calls",
            None,
            None,
        ));
    }

    parse_llm_payload(&output.assistant_message.text_content())
}

fn build_system_prompt() -> String {
    [
        "你是长篇小说创建期规划合同生成器。",
        "你的任务是直接产出 6 份可落盘的 Markdown 合同文档。",
        "只返回一个 JSON 对象，不要加解释，不要加代码块。",
        "JSON 必须包含这些键：story_brief, story_blueprint, narrative_contract, character_cards, foreshadow_registry, chapter_planning。",
        "每个键的值都必须是完整 Markdown 字符串。",
        "禁止输出“待补充”“待生成”“暂无”“TBD”等占位语。",
        "character_cards 必须包含“## 主角卡”。",
        "foreshadow_registry 必须是 Markdown 表格，至少 3 条伏笔。",
        "chapter_planning 必须包含“## 前五章章纲”和“## 前两章细纲”，至少出现 7 个“### 第”章节标题。",
    ]
    .join("\n")
}

fn build_user_prompt(
    project: &ProjectMetadata,
    resolved: &crate::application::command_usecases::inspiration::ResolvedConsensusSnapshot,
    create_handoff: &CreateProjectHandoffDraft,
) -> Result<String, AppError> {
    let prompt_input = json!({
        "project": {
            "name": project.name,
            "author": project.author,
            "description": project.description,
            "project_type": project.project_type,
            "target_total_words": project.target_total_words,
            "planned_volumes": project.planned_volumes,
            "target_words_per_volume": project.target_words_per_volume,
            "target_words_per_chapter": project.target_words_per_chapter,
            "narrative_pov": project.narrative_pov,
            "tone": project.tone,
            "audience": project.audience,
        },
        "consensus_snapshot": resolved,
        "create_handoff": create_handoff,
        "document_rules": {
            "story_brief_h1": PlanningDocId::StoryBrief.markdown_h1(),
            "story_blueprint_h1": PlanningDocId::StoryBlueprint.markdown_h1(),
            "narrative_contract_h1": PlanningDocId::NarrativeContract.markdown_h1(),
            "character_cards_h1": PlanningDocId::CharacterCards.markdown_h1(),
            "foreshadow_registry_h1": PlanningDocId::ForeshadowRegistry.markdown_h1(),
            "chapter_planning_h1": PlanningDocId::ChapterPlanning.markdown_h1(),
        }
    });

    let serialized = serde_json::to_string_pretty(&prompt_input).map_err(|error| {
        core_bundle_failure(
            "planning input serialization failed",
            Some(error.to_string()),
            None,
        )
    })?;

    Ok(format!(
        "请基于下面的项目创建输入生成 6 份正式规划合同。\n\n输出要求：\n1. 只返回一个 JSON 对象。\n2. 每个字段值都是完整 Markdown 文档。\n3. 文档内容必须具体、可直接落盘。\n4. 不要省略必要章节。\n\n输入：\n{serialized}"
    ))
}

fn parse_llm_payload(raw: &str) -> Result<LlmPlanningPayload, AppError> {
    let cleaned = strip_code_fences(raw).trim().to_string();
    let candidate = extract_json_object(&cleaned).unwrap_or(cleaned);
    serde_json::from_str::<LlmPlanningPayload>(&candidate).map_err(|error| {
        core_bundle_failure(
            "planning llm payload parse failed",
            Some(error.to_string()),
            Some(json!({ "raw_preview": candidate.chars().take(200).collect::<String>() })),
        )
    })
}

fn strip_code_fences(raw: &str) -> String {
    raw.replace("```json", "").replace("```", "")
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].to_string())
}

fn core_bundle_failure(
    message: &str,
    cause: Option<String>,
    extra: Option<serde_json::Value>,
) -> AppError {
    let mut details = json!({ "code": "CoreBundleGenerationFailed" });
    if let Some(cause) = cause {
        details["cause"] = serde_json::Value::String(cause);
    }
    if let Some(extra) = extra {
        if let Some(extra_obj) = extra.as_object() {
            for (key, value) in extra_obj {
                details[key] = value.clone();
            }
        }
    }

    AppError {
        code: ErrorCode::Internal,
        message: message.to_string(),
        details: Some(details),
        recoverable: Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::command_usecases::inspiration::{
        ConsensusField, ConsensusFieldId, ConsensusValue,
    };
    use crate::models::{ProjectBootstrapState, PROJECT_SCHEMA_VERSION};

    fn field(field_id: ConsensusFieldId, value: ConsensusValue) -> ConsensusField {
        ConsensusField {
            field_id,
            draft_value: Some(value),
            confirmed_value: None,
            locked: false,
            updated_at: 1,
            last_source_turn_id: None,
        }
    }

    fn project() -> ProjectMetadata {
        ProjectMetadata {
            schema_version: PROJECT_SCHEMA_VERSION,
            project_id: "project-1".to_string(),
            name: "暗潮协议".to_string(),
            author: "Tester".to_string(),
            description: Some("一部围绕代价交换展开的悬疑长篇".to_string()),
            cover_image: None,
            project_type: vec!["悬疑".to_string()],
            target_total_words: 300_000,
            planned_volumes: None,
            target_words_per_volume: None,
            target_words_per_chapter: None,
            narrative_pov: "third_limited".to_string(),
            tone: vec!["压迫".to_string()],
            audience: "general".to_string(),
            story_core: Some("秘密交易撬动旧秩序".to_string()),
            protagonist_anchor: Some("沈砚".to_string()),
            conflict_anchor: Some("规则以身份为代价".to_string()),
            origin_inspiration_session_id: Some("session-1".to_string()),
            planning_bundle_version: Some(1),
            bootstrap_state: ProjectBootstrapState::ScaffoldReady,
            bootstrap_updated_at: 1,
            created_at: 1,
            updated_at: 1,
            app_min_version: None,
            last_opened_at: Some(1),
        }
    }

    fn resolved_consensus(
    ) -> crate::application::command_usecases::inspiration::ResolvedConsensusSnapshot {
        let mut state = InspirationConsensusState::default();
        state.story_core = field(
            ConsensusFieldId::StoryCore,
            ConsensusValue::Text("秘密交易撬动旧秩序".to_string()),
        );
        state.premise = field(
            ConsensusFieldId::Premise,
            ConsensusValue::Text("一个习惯自保的人被迫卷入会吞噬身份的交易网络".to_string()),
        );
        state.genre_tone = field(
            ConsensusFieldId::GenreTone,
            ConsensusValue::List(vec!["悬疑".to_string(), "压迫感".to_string()]),
        );
        state.protagonist = field(
            ConsensusFieldId::Protagonist,
            ConsensusValue::Text("沈砚".to_string()),
        );
        state.core_conflict = field(
            ConsensusFieldId::CoreConflict,
            ConsensusValue::Text("想查明真相就必须继续喂养那套危险规则".to_string()),
        );
        state
            .resolve_for_variants()
            .expect("minimum consensus should be satisfied")
    }

    #[test]
    fn user_prompt_requests_chinese_contract_headings() {
        let handoff = CreateProjectHandoffDraft {
            name: "暗潮协议".to_string(),
            description: "一个习惯自保的人被迫卷入会吞噬身份的交易网络".to_string(),
            project_type: vec!["悬疑".to_string()],
            tone: vec!["压迫".to_string()],
            audience: "偏好强情节女性向悬疑的读者".to_string(),
            protagonist_seed: Some("沈砚，擅长隐藏真实意图".to_string()),
            counterpart_seed: None,
            world_seed: None,
            ending_direction: Some("主角必须亲手切断最诱人的捷径".to_string()),
        };

        let prompt = build_user_prompt(&project(), &resolved_consensus(), &handoff).expect("prompt");

        assert!(prompt.contains("\"story_brief_h1\": \"# 故事简报\""));
        assert!(prompt.contains("\"narrative_contract_h1\": \"# 叙事合同\""));
        assert!(prompt.contains("\"chapter_planning_h1\": \"# 章节规划\""));
    }
}
