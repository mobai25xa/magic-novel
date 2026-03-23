//! Tauri commands for Mission system
//!
//! Provides UI-facing commands: create, list, get_status, start, pause, cancel.

use serde::{Deserialize, Serialize};

mod dto;
mod knowledge_commands;
mod lifecycle;
mod lifecycle_commands;
mod m2_commands;
pub mod macro_commands;
mod review_commands;
mod review_gate;
mod runtime;
mod scheduler;

pub use dto::{
    MissionControlInput, MissionCreateInput, MissionCreateOutput, MissionGetStatusInput,
    MissionGetStatusOutput, MissionListInput, MissionStartInput,
};

pub use lifecycle_commands::{
    mission_cancel, mission_create, mission_get_status, mission_interrupt, mission_list,
    mission_pause, mission_recover, mission_resume, mission_resume_with_config, mission_start,
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
    mission_knowledge_list, mission_knowledge_repropose, mission_knowledge_rollback,
};

pub use macro_commands::{mission_macro_create, mission_macro_get_state};

use crate::mission::types::INTEGRATOR_FEATURE_ID;
use crate::mission::types::*;
use crate::mission::workflow_types::SummaryJobPolicy;

const REVIEW_FIXUP_MAX_ATTEMPTS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DelegateTransportMode {
    #[default]
    Process,
    InProcess,
}

impl DelegateTransportMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::InProcess => "in_process",
        }
    }
}

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
    parent_session_id: Option<String>,
    parent_turn_id: Option<u32>,
    delegate_transport: DelegateTransportMode,
}

fn append_explicit_summary_feature(features: &mut Vec<Feature>) {
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
        description: "Converge mission results and produce final mission summary".to_string(),
        skill: "integrator".to_string(),
        preconditions: Vec::new(),
        depends_on,
        expected_behavior: vec![
            "Produce final mission summary covering all features".to_string(),
            "Highlight unresolved failures and potential conflicts".to_string(),
        ],
        verification_steps: vec![
            "Summarize recorded mission results into final mission conclusion".to_string(),
            "List unresolved failures and blockers".to_string(),
        ],
        write_paths: Vec::new(),
    });
}

fn apply_summary_job_policy(summary_job_policy: &SummaryJobPolicy, features: &mut Vec<Feature>) {
    if *summary_job_policy == SummaryJobPolicy::ExplicitSummaryJob {
        append_explicit_summary_feature(features);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_feature(id: &str) -> Feature {
        Feature {
            id: id.to_string(),
            status: FeatureStatus::Pending,
            description: format!("feature {id}"),
            skill: String::new(),
            preconditions: Vec::new(),
            depends_on: Vec::new(),
            expected_behavior: Vec::new(),
            verification_steps: Vec::new(),
            write_paths: Vec::new(),
        }
    }

    #[test]
    fn summary_policy_does_not_append_job_by_default() {
        let mut features = vec![sample_feature("f1")];

        apply_summary_job_policy(&SummaryJobPolicy::ParentSessionSummary, &mut features);

        assert_eq!(features.len(), 1);
        assert!(features
            .iter()
            .all(|feature| feature.id != INTEGRATOR_FEATURE_ID));
    }

    #[test]
    fn summary_policy_can_append_explicit_summary_job() {
        let mut features = vec![sample_feature("f1"), sample_feature("f2")];

        apply_summary_job_policy(&SummaryJobPolicy::ExplicitSummaryJob, &mut features);

        let summary_feature = features
            .iter()
            .find(|feature| feature.id == INTEGRATOR_FEATURE_ID)
            .expect("summary feature should be appended");
        assert_eq!(
            summary_feature.depends_on,
            vec!["f1".to_string(), "f2".to_string()]
        );
    }
}
