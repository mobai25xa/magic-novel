//! Tauri commands for Mission system
//!
//! Provides UI-facing commands: create, list, get_status, start, pause, cancel.

mod dto;
mod knowledge_commands;
mod lifecycle_commands;
mod m2_commands;
mod review_commands;
mod review_gate;
mod runtime;
mod scheduler;

pub use dto::{
    MissionControlInput, MissionCreateInput, MissionCreateOutput, MissionGetStatusInput,
    MissionGetStatusOutput, MissionListInput, MissionStartInput,
};

pub use lifecycle_commands::{
    mission_cancel, mission_create, mission_get_status, mission_list, mission_pause,
    mission_resume, mission_start,
};

pub use review_commands::{
    mission_review_answer, mission_review_get_latest, mission_review_get_pending_decision,
    mission_review_list,
};

pub use m2_commands::{
    mission_contextpack_build, mission_contextpack_get_latest,
    mission_contextpack_rebuild_if_stale, mission_contextpack_status, mission_layer1_get,
    mission_layer1_upsert,
};

pub use knowledge_commands::{
    mission_knowledge_apply, mission_knowledge_decide, mission_knowledge_get_latest,
    mission_knowledge_rollback,
};

use crate::mission::types::INTEGRATOR_FEATURE_ID;
use crate::mission::types::*;

const REVIEW_FIXUP_MAX_ATTEMPTS: u32 = 2;

#[derive(Debug, Clone)]
struct MissionRunConfig {
    model: String,
    provider: String,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Clone)]
struct MissionStartConfig {
    run_config: MissionRunConfig,
    max_workers: usize,
}

fn append_integrator_feature_if_missing(features: &mut Vec<Feature>) {
    if features.iter().any(|f| f.id == INTEGRATOR_FEATURE_ID) {
        return;
    }

    let depends_on = features
        .iter()
        .map(|f| f.id.clone())
        .filter(|id| id != INTEGRATOR_FEATURE_ID)
        .collect::<Vec<_>>();

    features.push(Feature {
        id: INTEGRATOR_FEATURE_ID.to_string(),
        status: FeatureStatus::Pending,
        description: "Converge mission results and produce final handoff summary".to_string(),
        skill: "integrator".to_string(),
        preconditions: Vec::new(),
        depends_on,
        expected_behavior: vec![
            "Produce final handoff summary covering all features".to_string(),
            "Highlight unresolved failures and potential conflicts".to_string(),
        ],
        verification_steps: vec![
            "Summarize handoffs.jsonl into final mission conclusion".to_string(),
            "List unresolved failures and blockers".to_string(),
        ],
        write_paths: Vec::new(),
    });
}
