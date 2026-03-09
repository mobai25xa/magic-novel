use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecycleItemType {
    Novel,
    Chapter,
    Volume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecycleItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: RecycleItemType,
    pub name: String,
    pub origin: String,
    pub description: String,
    pub deleted_at: i64,
    pub days_remaining: i32,
}
