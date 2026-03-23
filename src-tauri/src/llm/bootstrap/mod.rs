mod generator;
mod live;
mod prompt;
mod template;
mod types;

pub use generator::{default_generator, BootstrapGenerator};
pub use prompt::{build_system_prompt, build_user_prompt};
pub use types::{
    BootstrapArtifactFailure, BootstrapArtifactKind, BootstrapChapterPlan,
    BootstrapCreativePayload, BootstrapGenerationResult, BootstrapPromptInput, BootstrapVolumePlan,
    GeneratedArtifact,
};
