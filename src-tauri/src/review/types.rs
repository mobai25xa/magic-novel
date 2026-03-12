use serde::{Deserialize, Serialize};

pub const REVIEW_SCHEMA_VERSION: i32 = 1;
pub const REVIEW_DECISION_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ReviewType {
    WordCount,
    Continuity,
    Logic,
    Character,
    Style,
    Terminology,
    Foreshadow,
    ObjectiveCompletion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOverallStatus {
    Pass,
    Warn,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSeverity {
    Info,
    Warn,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRecommendedAction {
    Accept,
    Revise,
    Escalate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRunInput {
    pub scope_ref: String,
    pub target_refs: Vec<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub review_types: Vec<ReviewType>,
    #[serde(default)]
    pub task_card_ref: Option<String>,
    #[serde(default)]
    pub context_pack_ref: Option<String>,
    #[serde(default)]
    pub effective_rules_fingerprint: Option<String>,
    #[serde(default)]
    pub severity_threshold: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub issue_id: String,
    pub review_type: ReviewType,
    pub severity: ReviewSeverity,
    pub summary: String,
    #[serde(default)]
    pub subject_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub confidence: ReviewConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    pub schema_version: i32,
    pub review_id: String,
    pub scope_ref: String,
    pub target_refs: Vec<String>,
    pub review_types: Vec<ReviewType>,
    pub overall_status: ReviewOverallStatus,
    pub issues: Vec<ReviewIssue>,
    #[serde(default)]
    pub evidence_summary: Vec<String>,
    pub recommended_action: ReviewRecommendedAction,
    pub generated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDecisionRequest {
    pub schema_version: i32,
    pub review_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<String>,
    pub scope_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_refs: Option<Vec<String>>,
    pub question: String,
    pub options: Vec<String>,
    #[serde(default)]
    pub context_summary: Vec<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDecisionAnswer {
    pub schema_version: i32,
    pub review_id: String,
    pub selected_option: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub answered_at: i64,
}
