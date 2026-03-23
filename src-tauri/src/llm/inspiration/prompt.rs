use crate::application::command_usecases::inspiration::ResolvedConsensusSnapshot;
use crate::models::AppError;

pub fn build_system_prompt() -> String {
    concat!(
        "You generate novel creation metadata variants. ",
        "Return only JSON. Do not use markdown fences. ",
        "Preserve the same confirmed story core across all variants. ",
        "Do not invent a new main plotline beyond the supplied consensus. ",
        "Produce exactly three variants with ids balanced, hook, and setting."
    )
    .to_string()
}

pub fn build_user_prompt(input: &ResolvedConsensusSnapshot) -> Result<String, AppError> {
    let payload = serde_json::json!({
        "consensus": input,
        "rules": {
            "same_story_core": true,
            "no_new_mainline": true,
            "variant_ids": ["balanced", "hook", "setting"],
        },
        "output_contract": {
            "shared_story_core": "string",
            "variants": [{
                "variant_id": "balanced | hook | setting",
                "label": "string",
                "title": "string",
                "one_liner": "string",
                "short_synopsis": "string",
                "long_synopsis": "string",
                "setting_summary": "string",
                "protagonist_summary": "string",
                "tags": ["string"],
                "tone": ["string"],
                "audience": "string",
                "protagonist_seed": "string",
                "counterpart_seed": "string",
                "world_seed": "string",
                "ending_direction": "string"
            }]
        }
    });

    serde_json::to_string(&payload).map_err(Into::into)
}
