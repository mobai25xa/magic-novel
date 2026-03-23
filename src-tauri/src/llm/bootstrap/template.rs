use async_trait::async_trait;

use crate::models::AppError;

use super::{
    BootstrapArtifactKind, BootstrapChapterPlan, BootstrapCreativePayload,
    BootstrapGenerationResult, BootstrapGenerator, BootstrapPromptInput, BootstrapVolumePlan,
};

pub struct TemplateBootstrapGenerator;

#[async_trait]
impl BootstrapGenerator for TemplateBootstrapGenerator {
    fn name(&self) -> &'static str {
        "template"
    }

    async fn generate(
        &self,
        input: BootstrapPromptInput,
        requested_kinds: Vec<BootstrapArtifactKind>,
    ) -> Result<BootstrapGenerationResult, AppError> {
        let payload = build_payload(&input);
        Ok(payload.materialize(&requested_kinds, self.name()))
    }
}

fn build_payload(input: &BootstrapPromptInput) -> BootstrapCreativePayload {
    let planned_volumes = input.planned_volumes.clamp(1, 8);
    let target_total_words = input.target_total_words.max(30_000);
    let target_words_per_volume = (target_total_words / planned_volumes.max(1)).max(15_000);
    let desired_chapter_target = input.target_words_per_chapter.max(2_000);

    let protagonist = input.protagonist_seed.clone().unwrap_or_else(|| {
        format!(
            "主角被迫卷入{}的核心冲突，最初只想自保，随后被责任逼到台前。",
            seed_subject(input)
        )
    });
    let counterpart = input.counterpart_seed.clone().unwrap_or_else(|| {
        "对手既代表旧秩序，也映照主角的潜在欲望，使两人的冲突兼具立场与情感张力。".to_string()
    });
    let world = input.world_seed.clone().unwrap_or_else(|| {
        format!(
            "世界运行在{}与日常秩序相互挤压的结构上，任何失控都会迅速改变角色关系与风险等级。",
            if input.genres.is_empty() {
                "隐秘规则".to_string()
            } else {
                input.genres.join("、")
            }
        )
    });
    let ending = input
        .ending_direction
        .clone()
        .unwrap_or_else(|| "结局必须让主角得到结果，同时付出真实代价。".to_string());

    let volumes = (0..planned_volumes)
        .map(|index| {
            build_volume_plan(
                index,
                planned_volumes,
                target_words_per_volume,
                desired_chapter_target,
            )
        })
        .collect::<Vec<_>>();

    BootstrapCreativePayload {
        story_blueprint: format!(
            "# Story Blueprint\n\n- 核心 premise：{}\n- 主冲突：主角必须在不断升级的风险里决定是否接管更大的责任。\n- 中段推进：每卷都让主角在策略、关系、代价之间做更困难的取舍。\n- 终局方向：{}\n",
            non_empty(&input.creation_brief, "待补充故事简介"),
            ending
        ),
        theme_notes: format!(
            "# Theme Notes\n\n- 主题关键词：责任、代价、选择、失控后的重建\n- 语气：{}\n- 叙事视角：{}\n- 目标读者：{}\n",
            if input.tone.is_empty() {
                "克制 / 紧张 / 角色驱动".to_string()
            } else {
                input.tone.join(" / ")
            },
            input.narrative_pov,
            input.audience
        ),
        protagonist_seed: format!("# Protagonist Seed\n\n{}\n", protagonist),
        counterpart_seed: format!("# Counterpart Seed\n\n{}\n", counterpart),
        world_summary: format!("# World Summary\n\n{}\n", world),
        main_plotline: format!(
            "# Main Plotline\n\n主线围绕“{}”展开：主角从被动卷入，到主动识别规则，再到重写局势，最终在代价中换取有限而真实的胜利。\n",
            seed_subject(input)
        ),
        volumes,
        recommended_next_action: if input.protagonist_seed.is_some() {
            "start_chapter_one".to_string()
        } else {
            "complete_protagonist_profile".to_string()
        },
    }
}

fn build_volume_plan(
    index: i32,
    planned_volumes: i32,
    target_words_per_volume: i32,
    desired_chapter_target: i32,
) -> BootstrapVolumePlan {
    let volume_number = index + 1;
    let chapters_per_volume = ((target_words_per_volume + desired_chapter_target - 1)
        / desired_chapter_target)
        .clamp(6, 12);
    let actual_chapter_target = (target_words_per_volume / chapters_per_volume.max(1)).max(1);
    let stage = volume_stage(volume_number as usize, planned_volumes as usize);

    let chapters = (0..chapters_per_volume)
        .map(|chapter_index| BootstrapChapterPlan {
            title: format!("{}·{}", stage.chapter_prefix, chapter_index + 1),
            summary: format!(
                "第{}卷第{}章：推进{}，并制造新的选择压力。",
                volume_number,
                chapter_index + 1,
                stage.volume_focus
            ),
            plot_goal: format!("围绕{}推进局势并留下新的悬念。", stage.chapter_goal),
            emotional_goal: stage.emotional_goal.to_string(),
            target_words: actual_chapter_target,
        })
        .collect::<Vec<_>>();

    BootstrapVolumePlan {
        title: stage.volume_title.to_string(),
        summary: format!(
            "本卷聚焦{}，让主角的行动边界被逐步抬高。",
            stage.volume_focus
        ),
        dramatic_goal: stage.dramatic_goal.to_string(),
        target_words: target_words_per_volume,
        chapters,
    }
}

fn seed_subject(input: &BootstrapPromptInput) -> String {
    if !input.description.trim().is_empty() {
        input.description.trim().to_string()
    } else if !input.creation_brief.trim().is_empty() {
        input.creation_brief.trim().to_string()
    } else {
        input.project_name.clone()
    }
}

fn non_empty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value.trim()
    }
}

#[derive(Clone, Copy)]
struct VolumeStage {
    volume_title: &'static str,
    volume_focus: &'static str,
    dramatic_goal: &'static str,
    chapter_prefix: &'static str,
    chapter_goal: &'static str,
    emotional_goal: &'static str,
}

fn volume_stage(index: usize, total: usize) -> VolumeStage {
    let stages = [
        VolumeStage {
            volume_title: "引爆与入局",
            volume_focus: "建立主角处境与核心威胁",
            dramatic_goal: "让主角无法继续置身事外",
            chapter_prefix: "异象",
            chapter_goal: "引爆危机",
            emotional_goal: "从迟疑切入到被迫警觉",
        },
        VolumeStage {
            volume_title: "试探与结盟",
            volume_focus: "扩大关系网并暴露隐性规则",
            dramatic_goal: "让主角第一次主动布局",
            chapter_prefix: "试探",
            chapter_goal: "搭建联盟",
            emotional_goal: "在试探中建立信任又保留戒心",
        },
        VolumeStage {
            volume_title: "反噬与失控",
            volume_focus: "让策略开始反噬并造成代价",
            dramatic_goal: "逼主角承认旧方法已经失效",
            chapter_prefix: "裂口",
            chapter_goal: "升级代价",
            emotional_goal: "让压抑转为失控前的拉扯",
        },
        VolumeStage {
            volume_title: "对决与改写",
            volume_focus: "主角重组资源并对抗核心对手",
            dramatic_goal: "完成最关键的立场选择",
            chapter_prefix: "对峙",
            chapter_goal: "逼近终局",
            emotional_goal: "从摇摆转为承担后果",
        },
        VolumeStage {
            volume_title: "余波与新秩序",
            volume_focus: "处理决战余波并建立新秩序",
            dramatic_goal: "把胜利写成带代价的完成态",
            chapter_prefix: "余波",
            chapter_goal: "结算后果",
            emotional_goal: "从高压过渡到有代价的平静",
        },
    ];

    if total <= stages.len() {
        stages[index.saturating_sub(1).min(stages.len() - 1)]
    } else {
        stages[index % stages.len()]
    }
}
