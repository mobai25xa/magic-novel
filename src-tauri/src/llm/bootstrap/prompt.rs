use crate::models::AppError;

use super::{BootstrapArtifactKind, BootstrapPromptInput};

pub fn build_system_prompt() -> String {
    concat!(
        "You are a novel bootstrap planner. ",
        "Return only JSON. Do not use markdown fences. ",
        "The JSON must match the requested schema exactly. ",
        "All generated artifacts are drafts, not final canon. ",
        "The volume and chapter plans must be internally consistent with the word-count budget."
    )
    .to_string()
}

pub fn build_user_prompt(
    input: &BootstrapPromptInput,
    requested_kinds: &[BootstrapArtifactKind],
) -> Result<String, AppError> {
    let payload = serde_json::json!({
        "requested_artifacts": requested_kinds.iter().map(BootstrapArtifactKind::as_str).collect::<Vec<_>>(),
        "project": {
            "name": input.project_name,
            "author": input.author,
            "description": input.description,
            "genres": input.genres,
            "target_total_words": input.target_total_words,
            "planned_volumes": input.planned_volumes,
            "target_words_per_volume": input.target_words_per_volume,
            "target_words_per_chapter": input.target_words_per_chapter,
            "narrative_pov": input.narrative_pov,
            "tone": input.tone,
            "audience": input.audience,
            "creation_brief": input.creation_brief,
            "protagonist_seed": input.protagonist_seed,
            "counterpart_seed": input.counterpart_seed,
            "world_seed": input.world_seed,
            "ending_direction": input.ending_direction,
        },
        "output_contract": {
            "story_blueprint": "string",
            "theme_notes": "string",
            "protagonist_seed": "string",
            "counterpart_seed": "string",
            "world_summary": "string",
            "main_plotline": "string",
            "volumes": [{
                "title": "string",
                "summary": "string",
                "dramatic_goal": "string",
                "target_words": "integer",
                "chapters": [{
                    "title": "string",
                    "summary": "string",
                    "plot_goal": "string",
                    "emotional_goal": "string",
                    "target_words": "integer"
                }]
            }],
            "recommended_next_action": "string"
        }
    });

    serde_json::to_string(&payload).map_err(Into::into)
}
