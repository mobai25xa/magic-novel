//! Core data types for the writing rules system.

use serde::{Deserialize, Serialize};

// ── RuleSet ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSetStatus {
    Draft,
    Accepted,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Volume,
    Chapter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChapterWordsConstraint {
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub max: Option<i32>,
    #[serde(default)]
    pub target: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleConstraints {
    #[serde(default)]
    pub chapter_words: Option<ChapterWordsConstraint>,
    #[serde(default)]
    pub style_template_id: Option<String>,
    #[serde(default)]
    pub pov: Option<String>,
    #[serde(default)]
    pub writing_notes: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleSet {
    pub schema_version: i32,
    pub ruleset_id: String,
    pub version: i32,
    pub status: RuleSetStatus,
    pub scope: RuleScope,
    pub scope_ref: String,
    pub constraints: RuleConstraints,

    #[serde(default)]
    pub validation_profile_id: Option<String>,

    #[serde(default)]
    pub previous_version: i32,
    #[serde(default)]
    pub effective_from_chapter: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

// ── StyleTemplate ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleTemplateStatus {
    Draft,
    Accepted,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleTemplateMeta {
    pub schema_version: i32,
    pub template_id: String,
    pub status: StyleTemplateStatus,
    pub summary: String,
    #[serde(default)]
    pub source_ref: Option<String>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct StyleTemplate {
    pub meta: StyleTemplateMeta,
    pub content: String,
}

// ── ValidationProfile ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckType {
    WordCountCheck,
    ContinuityCheck,
    LogicCheck,
    CharacterVoiceCheck,
    StyleTemplateCheck,
    TerminologyCheck,
    ForeshadowCheck,
    ObjectiveCompletionCheck,
    ForbiddenPatternCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeverityThreshold {
    None,
    Warn,
    Block,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationProfile {
    pub schema_version: i32,
    pub validation_profile_id: String,
    pub checks: Vec<CheckType>,
    #[serde(default = "default_severity_threshold")]
    pub severity_threshold: SeverityThreshold,
    #[serde(default)]
    pub strict_warn: bool,
    #[serde(default)]
    pub auto_fix_on_block: bool,
}

fn default_severity_threshold() -> SeverityThreshold {
    SeverityThreshold::Warn
}

// ── RuleSource (for EffectiveRules traceability) ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSource {
    pub scope: RuleScope,
    pub scope_ref: String,
    pub ruleset_id: String,
    pub version: i32,
}

// ── Conflict ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    pub field: String,
    pub description: String,
    pub sources: Vec<RuleSource>,
}

// ── EffectiveRules (the resolved output) ─────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveRules {
    pub scope_ref: String,
    #[serde(default)]
    pub chapter_words: Option<ChapterWordsConstraint>,
    #[serde(default)]
    pub style_template_id: Option<String>,
    #[serde(default)]
    pub pov: Option<String>,
    #[serde(default)]
    pub writing_notes: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
    #[serde(default)]
    pub validation_profile_id: Option<String>,
    pub rules_fingerprint: String,
    pub sources: Vec<RuleSource>,
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
}
