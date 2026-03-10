//! Mission system - ContextPack types (M2)
//!
//! Stored under: {project}/magic_novel/missions/{mission_id}/contextpacks/contextpack.json

use serde::{Deserialize, Serialize};

pub const CONTEXTPACK_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenBudget {
    Small,
    Medium,
    Large,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextPackCastNote {
    #[serde(default)]
    pub character_ref: String,
    #[serde(default)]
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_signals: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceSnippet {
    #[serde(default)]
    pub source_ref: String,
    #[serde(default)]
    pub snippet: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceRevision {
    #[serde(default)]
    pub r#ref: String,
    #[serde(default)]
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextPack {
    #[serde(default)]
    pub schema_version: i32,
    #[serde(default)]
    pub scope_ref: String,
    #[serde(default)]
    pub token_budget: TokenBudget,
    #[serde(default)]
    pub objective_summary: String,
    #[serde(default)]
    pub must_keep: Vec<String>,
    #[serde(default)]
    pub active_constraints: Vec<String>,
    #[serde(default)]
    pub key_facts: Vec<String>,
    #[serde(default)]
    pub cast_notes: Vec<ContextPackCastNote>,
    #[serde(default)]
    pub evidence_snippets: Vec<EvidenceSnippet>,
    #[serde(default)]
    pub style_rules: Vec<String>,
    #[serde(default)]
    pub review_targets: Vec<String>,
    #[serde(default)]
    pub risk_flags: Vec<String>,
    #[serde(default)]
    pub source_revisions: Vec<SourceRevision>,
    #[serde(default)]
    pub generated_at: i64,
}
