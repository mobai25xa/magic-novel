use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Lore,
    Prompt,
    Worldview,
    Outline,
    Character,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    pub imported_at: i64,
    pub importer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetNode {
    pub node_id: String,
    pub title: String,
    pub level: i32,
    pub content: String,
    pub children: Vec<AssetNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTree {
    pub schema_version: i32,
    pub id: String,
    pub kind: AssetKind,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<AssetSource>,
    pub root: AssetNode,
}
