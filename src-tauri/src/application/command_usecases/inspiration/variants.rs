use crate::llm::inspiration::default_generator;
use crate::models::AppError;

use super::types::{
    CreateProjectHandoffDraft, GenerateMetadataVariantsInput, GenerateMetadataVariantsOutput,
    MetadataVariant, MetadataVariantId,
};

pub async fn generate_metadata_variants(
    input: GenerateMetadataVariantsInput,
) -> Result<GenerateMetadataVariantsOutput, AppError> {
    let snapshot = input.consensus.resolve_for_variants().map_err(|missing| {
        AppError::invalid_argument(format!(
            "missing required consensus fields: {}",
            missing
                .iter()
                .map(|field| field.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    let output = default_generator().generate(snapshot.clone()).await?;
    validate_variant_output(output, &snapshot.story_core)
}

pub fn build_create_handoff(variant: &MetadataVariant) -> CreateProjectHandoffDraft {
    variant.to_create_handoff()
}

fn validate_variant_output(
    output: GenerateMetadataVariantsOutput,
    expected_story_core: &str,
) -> Result<GenerateMetadataVariantsOutput, AppError> {
    let normalized = output.normalize();

    if normalized.shared_story_core.trim() != expected_story_core.trim() {
        return Err(AppError::invalid_argument(
            "metadata variants must preserve the shared story core",
        ));
    }

    if normalized.variants.len() != MetadataVariantId::ordered().len() {
        return Err(AppError::invalid_argument(
            "metadata variants must include exactly balanced, hook, and setting",
        ));
    }

    for (index, expected_id) in MetadataVariantId::ordered().iter().enumerate() {
        let Some(variant) = normalized.variants.get(index) else {
            return Err(AppError::invalid_argument(
                "metadata variants are missing required variants",
            ));
        };

        if variant.variant_id != *expected_id {
            return Err(AppError::invalid_argument(
                "metadata variants must be ordered as balanced, hook, setting",
            ));
        }

        if variant.title.is_empty()
            || variant.one_liner.is_empty()
            || variant.short_synopsis.is_empty()
            || variant.long_synopsis.is_empty()
            || variant.setting_summary.is_empty()
            || variant.protagonist_summary.is_empty()
            || variant.protagonist_seed.is_empty()
            || variant.counterpart_seed.is_empty()
            || variant.world_seed.is_empty()
            || variant.ending_direction.is_empty()
        {
            return Err(AppError::invalid_argument(format!(
                "variant '{}' contains empty required fields",
                variant.variant_id.label()
            )));
        }
    }

    Ok(normalized)
}
