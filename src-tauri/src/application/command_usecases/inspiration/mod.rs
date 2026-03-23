mod consensus;
mod open_questions;
mod support;
#[cfg(test)]
mod tests;
mod types;
mod variants;

#[allow(unused_imports)]
pub use consensus::apply_consensus_patch;
#[allow(unused_imports)]
pub use open_questions::apply_open_questions_patch;
#[allow(unused_imports)]
pub use support::{
    create_inspiration_session, derive_inspiration_domain_state, ensure_inspiration_session_exists,
    load_inspiration_session_snapshot, save_inspiration_session_state,
    DerivedInspirationDomainState, LoadedInspirationSessionSnapshot,
    HYDRATION_STATUS_EVENT_REBUILT, HYDRATION_STATUS_NEW_SESSION, HYDRATION_STATUS_SNAPSHOT_LOADED,
};
#[allow(unused_imports)]
pub use types::{
    ApplyConsensusPatchInput, ApplyConsensusPatchOutput, ApplyOpenQuestionsPatchInput,
    ApplyOpenQuestionsPatchOutput, ConsensusField, ConsensusFieldId, ConsensusPatchOperation,
    ConsensusValue, CreateProjectHandoffDraft, GenerateMetadataVariantsInput,
    GenerateMetadataVariantsOutput, InspirationConsensusState, MetadataVariant, MetadataVariantId,
    OpenQuestion, OpenQuestionImportance, OpenQuestionStatus, OpenQuestionsPatchOperation,
    ResolvedConsensusSnapshot,
};
#[allow(unused_imports)]
pub use variants::{build_create_handoff, generate_metadata_variants};
