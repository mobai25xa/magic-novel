use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::application::command_usecases::inspiration::{
    CreateProjectHandoffDraft, InspirationConsensusState,
};
use crate::models::{
    AppError, ErrorCode, PlanningDocEntry, PlanningDocId, PlanningManifest, ProjectMetadata,
    VolumeMetadata, PLANNING_MANIFEST_REL_PATH,
};
use crate::services::{ensure_dir, read_json, write_file, write_json};

const MANUSCRIPTS_DIR: &str = "manuscripts";

#[derive(Debug, Clone)]
pub struct PlanningBundle {
    pub docs: Vec<PlanningDoc>,
    pub manifest: PlanningManifest,
    pub volume_plans: Vec<PlanningVolumePlan>,
}

#[derive(Debug, Clone)]
pub struct PlanningDoc {
    pub doc_id: PlanningDocId,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanningVolumePlan {
    pub title: String,
    pub summary: String,
    pub dramatic_goal: String,
    pub target_words: i32,
}

#[derive(Debug, Clone)]
pub struct PlanningGenerationMetadata {
    pub generation_source: String,
    pub generation_provider: Option<String>,
    pub generation_model: Option<String>,
}

pub fn build_deterministic_planning_bundle(
    project: &ProjectMetadata,
    consensus_snapshot: &InspirationConsensusState,
    create_handoff: &CreateProjectHandoffDraft,
    generation: PlanningGenerationMetadata,
) -> Result<PlanningBundle, AppError> {
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

    let story_core = resolved.story_core;
    let premise = resolved.premise;
    let genre_tone = join_or_fallback(
        &resolved.genre_tone,
        "悬念驱动 / 关系推进 / 情绪递进".to_string(),
    );
    let protagonist_anchor = resolved.protagonist;
    let worldview = resolved.worldview.unwrap_or_else(|| {
        create_handoff
            .world_seed
            .clone()
            .unwrap_or_else(|| "故事发生在一套看似稳定、实则持续吞噬个体余地的秩序里".to_string())
    });
    let core_conflict = resolved.core_conflict;
    let audience = resolved
        .audience
        .or_else(|| non_empty_string(&create_handoff.audience))
        .unwrap_or_else(|| "面向喜欢强冲突与人物张力的通用读者".to_string());
    let selling_points = join_or_fallback(
        &resolved.selling_points,
        format!(
            "高压开局、{}的角色张力、围绕“{}”逐步加码",
            protagonist_anchor, story_core
        ),
    );
    let ending_direction = resolved
        .ending_direction
        .or_else(|| create_handoff.ending_direction.clone())
        .unwrap_or_else(|| "主角必须亲手切断最诱人的捷径，并承担由此产生的后果".to_string());
    let counterpart_anchor = create_handoff.counterpart_seed.clone().unwrap_or_else(|| {
        format!(
            "一位既能帮助主角、又会放大“{}”代价的人物或势力",
            core_conflict
        )
    });
    let protagonist_seed = create_handoff.protagonist_seed.clone().unwrap_or_else(|| {
        format!(
            "{}，习惯隐藏真实意图，却无法继续回避选择代价",
            protagonist_anchor
        )
    });

    let chapter_titles = [
        "异常信号进入视野",
        "第一次选择带来代价",
        "盟友与误判同时出现",
        "真相露出危险轮廓",
        "主角主动抢回节奏",
    ];
    let chapter_pivots = [
        format!(
            "{}在日常秩序中先察觉到与“{}”相关的异常",
            protagonist_anchor, story_core
        ),
        format!("{}为了接近答案，主动跨过第一道边界", protagonist_anchor),
        format!(
            "{}与{}形成不稳定合作",
            protagonist_anchor, counterpart_anchor
        ),
        format!("世界规则的代价被具象化，{}不再只是抽象威胁", core_conflict),
        format!("{}决定不再被动应对，而是反向设计局面", protagonist_anchor),
    ];

    let story_brief = format!(
        "{}\n\n## 工作标题\n{}\n\n## 一句话定位\n{}\n\n## 创作摘要\n{}\n\n## 故事核心\n{}\n\n## 核心冲突\n{}\n\n## 目标读者\n{}\n\n## 类型与气质\n{}\n\n## 作品卖点\n{}\n",
        PlanningDocId::StoryBrief.markdown_h1(),
        project.name,
        premise,
        create_handoff.description.trim(),
        story_core,
        core_conflict,
        audience,
        genre_tone,
        selling_points,
    );

    let story_blueprint = format!(
        "{}\n\n## 叙事主轴\n{}\n\n## 开局抓手\n{}在一个看似稳定的秩序里被迫注意到与“{}”有关的裂缝，故事从异常而不是说明开始。\n\n## 中段推进\n主线通过连续升级的选择推进：每一次靠近真相，都会让{}付出更高的人际与身份代价。\n\n## 中点翻转\n主角意识到问题并不是单点敌人，而是与“{}”绑定的整体规则或结构。\n\n## 终局方向\n{}\n\n## 世界基底\n{}\n\n## 角色对撞\n- 主角：{}\n- 对位角色：{}\n- 冲突焦点：{}\n",
        PlanningDocId::StoryBlueprint.markdown_h1(),
        story_core,
        protagonist_anchor,
        core_conflict,
        protagonist_anchor,
        core_conflict,
        ending_direction,
        worldview,
        protagonist_seed,
        counterpart_anchor,
        core_conflict,
    );

    let narrative_contract = format!(
        "{}\n\n## 读者承诺\n本书承诺提供一条围绕“{}”逐步升级的主线，让读者始终清楚每一次选择会改变什么。\n\n## 视角合同\n- 主视角：{}\n- 允许范围：只展示主角当前知道、误判或故意隐瞒的信息\n- 叙事目标：让读者跟着主角一起承担判断成本，而不是从上帝视角提前获得答案\n\n## 节奏合同\n- 开篇三章必须持续制造前进压力\n- 每个转折都要落回“{}”的代价\n- 中段不能用说明替代行动，冲突必须通过角色决策推进\n\n## 情绪合同\n- 基调：{}\n- 情绪曲线：从不安到逼近失控，再到主动掌控\n\n## 世界规则合同\n{}\n\n## 禁止项\n- 不用旁白一次性解释全部设定\n- 不让关键矛盾靠巧合自行消失\n- 不让主角在没有代价的情况下获得最终优势\n",
        PlanningDocId::NarrativeContract.markdown_h1(),
        story_core,
        project.narrative_pov,
        core_conflict,
        genre_tone,
        worldview,
    );

    let character_cards = format!(
        "{}\n\n## 主角卡\n- 名称锚点：{}\n- 初始状态：{}\n- 外在目标：查清并处理“{}”带来的现实威胁\n- 内在伤口：在关键关系或旧选择上仍有未完成的亏欠感\n- 成长方向：从被动自保转向主动承担后果\n\n## 对位角色卡\n- 名称锚点：{}\n- 剧情作用：既提供通路，也不断抬高主角的判断成本\n- 与主角的张力：合作与试探并存，任何信任都不是无条件的\n- 关键秘密：他或她与“{}”存在比表面更深的绑定关系\n\n## 世界压力卡\n- 核心环境：{}\n- 压力来源：秩序表面稳定，但维系方式本身正在制造新一轮危机\n",
        PlanningDocId::CharacterCards.markdown_h1(),
        protagonist_anchor,
        protagonist_seed,
        core_conflict,
        counterpart_anchor,
        core_conflict,
        worldview,
    );

    let foreshadow_registry = format!(
        "{}\n\n| 编号 | 伏笔投放 | 表层呈现 | 后续回收 |\n| --- | --- | --- | --- |\n| F1 | 开篇异常信号 | 与“{}”相关的细节在第一章提前出现，但角色只当作偶发噪音 | 中段证明这不是偶然，而是整个结构开始失衡的第一声警报 |\n| F2 | 人物关系错位 | {}在首次合作时故意少说了一层真相 | 第四章揭示其隐瞒不是背叛，而是与自身代价绑定 |\n| F3 | 规则的代价 | 世界规则带来便利的同时，也悄悄吞噬某种身份、记忆或关系 | 终局阶段主角必须决定是否继续接受这套代价交换 |\n",
        PlanningDocId::ForeshadowRegistry.markdown_h1(),
        core_conflict,
        counterpart_anchor,
    );

    let chapter_planning = format!(
        "{}\n\n## 前五章章纲\n\n### 第1章：{}\n- 剧情任务：{}\n- 对抗焦点：主角以为还能维持原状，但异常已经逼近私人生活\n- 章节结果：主角被迫承认问题已经进入自己可见范围\n\n### 第2章：{}\n- 剧情任务：{}\n- 对抗焦点：第一次主动行动带来即时收益，也暴露新的关系裂口\n- 章节结果：主角得到线索，但站位因此更危险\n\n### 第3章：{}\n- 剧情任务：{}\n- 对抗焦点：合作建立在不完整真相之上，信任每前进一步都伴随试探\n- 章节结果：主角获得进入主线核心的通道，同时欠下更大的判断债务\n\n### 第4章：{}\n- 剧情任务：{}\n- 对抗焦点：问题规模从个人层面升级到结构层面，退路开始消失\n- 章节结果：主角确认继续旁观只会让代价扩大\n\n### 第5章：{}\n- 剧情任务：{}\n- 对抗焦点：主角第一次主动设计局面，不再只对外界做被动反应\n- 章节结果：故事进入可持续扩展的中段推进态势\n\n## 前两章细纲\n\n### 第1章细纲：{}\n1. 场景一：用一个具体异常切入，让主角先处理表层问题，再意识到它指向“{}”。\n2. 场景二：主角试图用旧经验压住局面，却因为信息缺口做出第一次误判。\n3. 场景三：章节结尾抛出新的不可回避线索，迫使主角离开舒适区。\n\n### 第2章细纲：{}\n1. 场景一：主角根据上一章线索采取行动，表面推进顺利，实则踩进更深层的规则代价。\n2. 场景二：{}以帮助者或阻断者身份出现，关系张力被正式点燃。\n3. 场景三：章节结尾给出一次看似有利、实则改变站位的选择，主角只能承担后果继续前进。\n",
        PlanningDocId::ChapterPlanning.markdown_h1(),
        chapter_titles[0],
        chapter_pivots[0],
        chapter_titles[1],
        chapter_pivots[1],
        chapter_titles[2],
        chapter_pivots[2],
        chapter_titles[3],
        chapter_pivots[3],
        chapter_titles[4],
        chapter_pivots[4],
        chapter_titles[0],
        core_conflict,
        chapter_titles[1],
        counterpart_anchor,
    );

    let docs = vec![
        PlanningDoc {
            doc_id: PlanningDocId::StoryBrief,
            content: story_brief,
        },
        PlanningDoc {
            doc_id: PlanningDocId::StoryBlueprint,
            content: story_blueprint,
        },
        PlanningDoc {
            doc_id: PlanningDocId::NarrativeContract,
            content: narrative_contract,
        },
        PlanningDoc {
            doc_id: PlanningDocId::CharacterCards,
            content: character_cards,
        },
        PlanningDoc {
            doc_id: PlanningDocId::ForeshadowRegistry,
            content: foreshadow_registry,
        },
        PlanningDoc {
            doc_id: PlanningDocId::ChapterPlanning,
            content: chapter_planning,
        },
    ];

    build_planning_bundle_from_core_docs(project, docs, generation)
}

pub fn build_planning_bundle_from_core_docs(
    project: &ProjectMetadata,
    mut docs: Vec<PlanningDoc>,
    generation: PlanningGenerationMetadata,
) -> Result<PlanningBundle, AppError> {
    validate_generated_docs(&docs)?;

    let volume_plans = project
        .planned_volumes
        .filter(|planned| *planned > 0)
        .map(build_volume_plans)
        .unwrap_or_default();

    let mut optional_outputs = Vec::new();
    if !volume_plans.is_empty() {
        docs.push(PlanningDoc {
            doc_id: PlanningDocId::VolumePlan,
            content: render_volume_plan(&volume_plans),
        });
        optional_outputs.push(PlanningDocId::VolumePlan.as_str().to_string());
    }

    let updated_at = chrono::Utc::now().timestamp_millis();
    let manifest_docs = docs
        .iter()
        .map(|doc| {
            PlanningDocEntry::new(doc.doc_id, generation.generation_source.clone(), updated_at)
        })
        .collect();
    let manifest = PlanningManifest::new(
        manifest_docs,
        optional_outputs,
        Some(generation.generation_source.clone()),
        generation.generation_provider.clone(),
        generation.generation_model.clone(),
        updated_at,
    );

    Ok(PlanningBundle {
        docs,
        manifest,
        volume_plans,
    })
}

pub fn persist_planning_bundle(
    project_path: &Path,
    bundle: &PlanningBundle,
) -> Result<(), AppError> {
    for doc in &bundle.docs {
        let file_path = project_path.join(doc.doc_id.relative_path());
        write_file(&file_path, &doc.content)?;
    }

    write_json(
        &project_path.join(PLANNING_MANIFEST_REL_PATH),
        &bundle.manifest,
    )?;
    persist_volume_shells(project_path, &bundle.volume_plans)?;
    Ok(())
}

pub fn load_planning_manifest(project_path: &Path) -> Result<Option<PlanningManifest>, AppError> {
    let manifest_path = project_path.join(PLANNING_MANIFEST_REL_PATH);
    if !manifest_path.exists() {
        return Ok(None);
    }

    let mut manifest: PlanningManifest = read_json(&manifest_path)?;
    manifest.refresh_derived_fields();
    Ok(Some(manifest))
}

pub fn ensure_planning_manifest_on_open(
    project_path: &Path,
) -> Result<Option<PlanningManifest>, AppError> {
    if let Some(manifest) = load_planning_manifest(project_path)? {
        return Ok(Some(manifest));
    }

    super::planning_status::refresh_planning_manifest_impl(project_path).map(Some)
}

fn persist_volume_shells(
    project_path: &Path,
    volume_plans: &[PlanningVolumePlan],
) -> Result<(), AppError> {
    let manuscripts_path = project_path.join(MANUSCRIPTS_DIR);
    ensure_dir(&manuscripts_path)?;

    for plan in volume_plans {
        let mut volume = VolumeMetadata::new(plan.title.clone());
        volume.summary = Some(plan.summary.clone());
        volume.dramatic_goal = plan.dramatic_goal.clone();
        volume.target_words = plan.target_words;
        let volume_dir = manuscripts_path.join(&volume.volume_id);
        ensure_dir(&volume_dir)?;
        write_json(&volume_dir.join("volume.json"), &volume)?;
    }

    Ok(())
}

fn validate_generated_docs(docs: &[PlanningDoc]) -> Result<(), AppError> {
    for doc in docs {
        let content = doc.content.trim();
        if content.is_empty()
            || content.contains("待补充")
            || content.contains("待生成")
            || content.contains("暂无")
        {
            return Err(AppError {
                code: ErrorCode::Internal,
                message: format!("planning doc {} failed quality gate", doc.doc_id.as_str()),
                details: Some(json!({
                    "code": "CoreBundleGenerationFailed",
                    "doc_id": doc.doc_id.as_str(),
                })),
                recoverable: Some(false),
            });
        }
    }

    let chapter_planning = docs
        .iter()
        .find(|doc| matches!(doc.doc_id, PlanningDocId::ChapterPlanning))
        .ok_or_else(|| AppError {
            code: ErrorCode::Internal,
            message: "missing chapter planning document".to_string(),
            details: Some(json!({ "code": "CoreBundleGenerationFailed" })),
            recoverable: Some(false),
        })?;

    let foreshadow_registry = docs
        .iter()
        .find(|doc| matches!(doc.doc_id, PlanningDocId::ForeshadowRegistry))
        .ok_or_else(|| AppError {
            code: ErrorCode::Internal,
            message: "missing foreshadow registry document".to_string(),
            details: Some(json!({ "code": "CoreBundleGenerationFailed" })),
            recoverable: Some(false),
        })?;

    let character_cards = docs
        .iter()
        .find(|doc| matches!(doc.doc_id, PlanningDocId::CharacterCards))
        .ok_or_else(|| AppError {
            code: ErrorCode::Internal,
            message: "missing character cards document".to_string(),
            details: Some(json!({ "code": "CoreBundleGenerationFailed" })),
            recoverable: Some(false),
        })?;

    if chapter_planning.content.matches("### 第").count() < 7 {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: "chapter planning did not include enough outline detail".to_string(),
            details: Some(json!({
                "code": "CoreBundleGenerationFailed",
                "doc_id": "chapter_planning"
            })),
            recoverable: Some(false),
        });
    }

    if foreshadow_registry.content.matches("| F").count() < 3 {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: "foreshadow registry did not include enough entries".to_string(),
            details: Some(json!({
                "code": "CoreBundleGenerationFailed",
                "doc_id": "foreshadow_registry"
            })),
            recoverable: Some(false),
        });
    }

    if !character_cards.content.contains("## 主角卡") {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: "character cards did not include protagonist card".to_string(),
            details: Some(json!({
                "code": "CoreBundleGenerationFailed",
                "doc_id": "character_cards"
            })),
            recoverable: Some(false),
        });
    }

    Ok(())
}

fn build_volume_plans(planned_volumes: i32) -> Vec<PlanningVolumePlan> {
    let clamped = planned_volumes.clamp(1, 8);
    let target_words = (300_000 / clamped.max(1)).max(20_000);

    (0..clamped)
        .map(|index| PlanningVolumePlan {
            title: format!("卷{}", index + 1),
            summary: match index {
                0 => "建立世界规则与角色动机，让主冲突完成第一次具象化。".to_string(),
                1 => "把误判扩展成结构性危机，让角色关系真正进入对撞。".to_string(),
                _ if index == clamped - 1 => {
                    "把所有伏笔收束到终局选择，让人物完成代价明确的闭环。".to_string()
                }
                _ => "持续升级代价与选择，让主线从局部事件扩展到整体结构。".to_string(),
            },
            dramatic_goal: match index {
                0 => "开局立钩并逼主角离开旧秩序".to_string(),
                _ if index == clamped - 1 => "终局摊牌并完成价值兑现".to_string(),
                _ => "不断加码冲突并压缩退路".to_string(),
            },
            target_words,
        })
        .collect()
}

fn render_volume_plan(volume_plans: &[PlanningVolumePlan]) -> String {
    let mut output = String::from(PlanningDocId::VolumePlan.markdown_h1());
    output.push('\n');
    for (index, plan) in volume_plans.iter().enumerate() {
        output.push_str(&format!(
            "\n## 卷{}：{}\n- 目标字数：{}\n- 叙事任务：{}\n- 摘要：{}\n",
            index + 1,
            plan.title,
            plan.target_words,
            plan.dramatic_goal,
            plan.summary
        ));
    }
    output
}

fn join_or_fallback(values: &[String], fallback: String) -> String {
    if values.is_empty() {
        fallback
    } else {
        values.join(" / ")
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
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

    fn consensus() -> InspirationConsensusState {
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
        state.audience = field(
            ConsensusFieldId::Audience,
            ConsensusValue::Text("偏好强情节女性向悬疑的读者".to_string()),
        );
        state
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

    #[test]
    fn deterministic_bundle_creates_manifest_with_blockers() {
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

        let bundle = build_deterministic_planning_bundle(
            &project(),
            &consensus(),
            &handoff,
            PlanningGenerationMetadata {
                generation_source: "deterministic_fallback".to_string(),
                generation_provider: None,
                generation_model: None,
            },
        )
        .expect("bundle");

        assert_eq!(bundle.docs.len(), 6);
        assert_eq!(bundle.manifest.bundle_status, "ready");
        assert!(!bundle.manifest.writing_readiness.can_start);
        assert_eq!(
            bundle.manifest.recommended_next_doc,
            PlanningDocId::NarrativeContract.relative_path()
        );
        for doc in &bundle.docs {
            assert!(doc.content.starts_with(doc.doc_id.markdown_h1()));
        }
    }
}
