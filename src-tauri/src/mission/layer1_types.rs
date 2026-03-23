//! Mission system - Layer1 task artifacts (M2)
//!
//! Stored under: {project}/magic_novel/missions/{mission_id}/layer1/

use serde::{Deserialize, Serialize};

pub const LAYER1_SCHEMA_VERSION: i32 = 1;

// ── Kinds (used by UI upsert) ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Layer1ArtifactKind {
    ChapterCard,
    RecentFacts,
    ActiveCast,
    ActiveForeshadowing,
    PreviousSummary,
    RiskLedger,
}

// ── Snapshot (mission_layer1_get) ───────────────────────────────

/// Combined Layer1 snapshot.
///
/// Contract: missing artifacts serialize as `null`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Layer1Snapshot {
    pub chapter_card: Option<ChapterCard>,
    pub recent_facts: Option<RecentFacts>,
    pub active_cast: Option<ActiveCast>,
    pub active_foreshadowing: Option<serde_json::Value>,
    pub previous_summary: Option<serde_json::Value>,
    pub risk_ledger: Option<serde_json::Value>,
}

// ── chapter_card.json ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowKind {
    Micro,
    Chapter,
    Arc,
    Book,
}

impl Default for ChapterWorkflowKind {
    fn default() -> Self {
        Self::Chapter
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChapterCardStatus {
    Draft,
    Active,
    Blocked,
    Completed,
}

impl Default for ChapterCardStatus {
    fn default() -> Self {
        Self::Draft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterCard {
    #[serde(default)]
    pub schema_version: i32,
    #[serde(default)]
    pub scope_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_locator: Option<String>,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub workflow_kind: ChapterWorkflowKind,
    #[serde(default)]
    pub hard_constraints: Vec<String>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub status: ChapterCardStatus,
    #[serde(default)]
    pub updated_at: i64,
    // Rule-P3: chapter binding — set at draft-write time, read at review time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules_sources: Vec<BoundRuleSource>,
    // Optional redundant fields for debug / DevC reads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_validation_profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound_style_template_id: Option<String>,
}

/// A single ruleset version reference bound to a chapter.
/// Mirrors `RuleSource` from writing_rules, kept here to avoid circular deps.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BoundRuleSource {
    pub scope: String,
    pub scope_ref: String,
    pub ruleset_id: String,
    pub version: i32,
}

// ── recent_facts.json ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactConfidence {
    Accepted,
    Proposed,
}

impl Default for FactConfidence {
    fn default() -> Self {
        Self::Proposed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentFact {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub source_ref: String,
    #[serde(default)]
    pub confidence: FactConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentFacts {
    #[serde(default)]
    pub schema_version: i32,
    #[serde(default)]
    pub scope_ref: String,
    #[serde(default)]
    pub facts: Vec<RecentFact>,
    #[serde(default)]
    pub updated_at: i64,
}

// ── active_cast.json ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveCastEntry {
    #[serde(default)]
    pub character_ref: String,
    #[serde(default)]
    pub current_state_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_keep_voice_signals: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveCast {
    #[serde(default)]
    pub schema_version: i32,
    #[serde(default)]
    pub scope_ref: String,
    #[serde(default)]
    pub cast: Vec<ActiveCastEntry>,
    #[serde(default)]
    pub updated_at: i64,
}
