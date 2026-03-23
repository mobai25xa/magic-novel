use async_trait::async_trait;

use crate::application::command_usecases::inspiration::{
    GenerateMetadataVariantsOutput, MetadataVariant, MetadataVariantId, ResolvedConsensusSnapshot,
};
use crate::models::AppError;

use super::generator::MetadataVariantGenerator;

pub struct TemplateMetadataVariantGenerator;

#[async_trait]
impl MetadataVariantGenerator for TemplateMetadataVariantGenerator {
    fn name(&self) -> &'static str {
        "template"
    }

    async fn generate(
        &self,
        input: ResolvedConsensusSnapshot,
    ) -> Result<GenerateMetadataVariantsOutput, AppError> {
        let story_core = input.story_core.clone();
        let premise = input.premise.clone();
        let protagonist = input.protagonist.clone();
        let core_conflict = input.core_conflict.clone();
        let audience = input
            .audience
            .clone()
            .unwrap_or_else(|| "待确认读者".to_string());
        let ending_direction = input
            .ending_direction
            .clone()
            .unwrap_or_else(|| "结局应回应核心冲突并保留真实代价。".to_string());
        let worldview = input
            .worldview
            .clone()
            .unwrap_or_else(|| format!("故事围绕“{}”展开独特规则与代价。", story_core));
        let tags = combine_tags(&input.genre_tone, &input.selling_points);
        let tone = if input.genre_tone.is_empty() {
            vec!["克制".to_string()]
        } else {
            input.genre_tone.clone()
        };
        let counterpart_seed = format!(
            "对手会围绕“{}”代表主角最不愿承认的立场，并持续逼迫主角选边。",
            core_conflict
        );
        let world_seed = format!("{worldview} 任何人想改写局面，都必须先承担相应代价。");

        Ok(GenerateMetadataVariantsOutput {
            shared_story_core: story_core.clone(),
            variants: vec![
                MetadataVariant {
                    variant_id: MetadataVariantId::Balanced,
                    label: MetadataVariantId::Balanced.label().to_string(),
                    title: candidate_title(&story_core, "平衡"),
                    one_liner: premise.clone(),
                    short_synopsis: format!(
                        "{} 主角{}，并在“{}”里不断逼近抉择。",
                        story_core, protagonist, core_conflict
                    ),
                    long_synopsis: format!(
                        "{}\n\n故事从“{}”切入，主角{}。随着局势推进，{}。整部作品的张力建立在“{}”之上，最终走向“{}”。",
                        story_core,
                        premise,
                        protagonist,
                        worldview,
                        core_conflict,
                        ending_direction
                    ),
                    setting_summary: worldview.clone(),
                    protagonist_summary: protagonist.clone(),
                    tags: tags.clone(),
                    tone: tone.clone(),
                    audience: audience.clone(),
                    protagonist_seed: protagonist.clone(),
                    counterpart_seed: counterpart_seed.clone(),
                    world_seed: world_seed.clone(),
                    ending_direction: ending_direction.clone(),
                },
                MetadataVariant {
                    variant_id: MetadataVariantId::Hook,
                    label: MetadataVariantId::Hook.label().to_string(),
                    title: candidate_title(&core_conflict, "钩子"),
                    one_liner: format!(
                        "当{}，主角必须决定{}。",
                        premise, core_conflict
                    ),
                    short_synopsis: format!(
                        "这是一部围绕“{}”展开的高压故事。主角{}，每一次行动都会把他推向更危险的选择。",
                        story_core, protagonist
                    ),
                    long_synopsis: format!(
                        "如果说这本书的核心钩子是什么，那就是“{}”。\n\n主角{}，起初只想维持局面，却在“{}”的连锁反应里被迫站上台前。{} 最终，故事必须以“{}”收束。",
                        story_core,
                        protagonist,
                        core_conflict,
                        worldview,
                        ending_direction
                    ),
                    setting_summary: format!(
                        "{} 设定的作用不是点缀，而是不断抬高选择成本。",
                        worldview
                    ),
                    protagonist_summary: format!(
                        "{} 他最大的弱点，也正是故事最强的钩子。",
                        protagonist
                    ),
                    tags: tags.clone(),
                    tone: tone.clone(),
                    audience: audience.clone(),
                    protagonist_seed: protagonist.clone(),
                    counterpart_seed: counterpart_seed.clone(),
                    world_seed: world_seed.clone(),
                    ending_direction: ending_direction.clone(),
                },
                MetadataVariant {
                    variant_id: MetadataVariantId::Setting,
                    label: MetadataVariantId::Setting.label().to_string(),
                    title: candidate_title(&worldview, "设定"),
                    one_liner: format!(
                        "在{}的世界里，{}。",
                        worldview, premise
                    ),
                    short_synopsis: format!(
                        "作品以“{}”为核心设定卖点，让主角{}，在“{}”中不断碰撞规则边界。",
                        worldview, protagonist, core_conflict
                    ),
                    long_synopsis: format!(
                        "这部作品最鲜明的吸引力，在于“{}”这套设定本身。\n\n围绕“{}”，主角{}，并逐步发现规则背后的真正代价。故事主线始终是“{}”，只是通过更强的设定感去包装和放大。终局仍将落在“{}”。",
                        worldview,
                        story_core,
                        protagonist,
                        core_conflict,
                        ending_direction
                    ),
                    setting_summary: worldview,
                    protagonist_summary: protagonist.clone(),
                    tags,
                    tone,
                    audience,
                    protagonist_seed: protagonist,
                    counterpart_seed,
                    world_seed,
                    ending_direction,
                },
            ],
        })
    }
}

fn combine_tags(genre_tone: &[String], selling_points: &[String]) -> Vec<String> {
    let mut tags = Vec::new();
    for item in genre_tone.iter().chain(selling_points.iter()) {
        let trimmed = item.trim();
        if trimmed.is_empty() || tags.iter().any(|existing| existing == trimmed) {
            continue;
        }
        tags.push(trimmed.to_string());
    }
    tags
}

fn candidate_title(seed: &str, suffix: &str) -> String {
    let compact = seed
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .take(10)
        .collect::<String>();
    if compact.is_empty() {
        format!("未定名·{suffix}")
    } else {
        format!("{compact}·{suffix}版")
    }
}
