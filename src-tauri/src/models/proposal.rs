use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Generated,
    Accepted,
    PartiallyAccepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalTarget {
    #[serde(rename = "type")]
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalContextRefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lore_asset_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_asset_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalOutput {
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiptap_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProposal {
    pub schema_version: i32,
    pub proposal_id: String,
    pub chapter_id: String,
    pub status: ProposalStatus,
    pub prompt: String,
    pub target: ProposalTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_refs: Option<ProposalContextRefs>,
    pub model: ProposalModel,
    pub output: ProposalOutput,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<i64>,
}
