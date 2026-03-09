use serde::{Deserialize, Serialize};

fn normalize_genres(mut input: Vec<String>) -> Vec<String> {
    for s in input.iter_mut() {
        *s = s.trim().to_string();
    }
    input.retain(|s| !s.is_empty());

    let mut out: Vec<String> = Vec::new();
    for s in input {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

fn deserialize_project_type<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    let genres = match value {
        serde_json::Value::Null => vec![],
        serde_json::Value::String(s) => vec![s],
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => vec![],
    };

    let mut normalized = normalize_genres(genres);

    // Drop legacy project-type values from older versions.
    // Previous values were: novel/script/essay/diary/business/academic/other
    normalized.retain(|g| {
        !matches!(
            g.as_str(),
            "novel" | "script" | "essay" | "diary" | "business" | "academic" | "other"
        )
    });

    Ok(normalized)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub schema_version: i32,
    pub project_id: String,
    pub name: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,

    // NOTE: 兼容旧数据：过去这里是单值字符串（novel/script/...），现在是小说题材标签（多选）。
    #[serde(default, deserialize_with = "deserialize_project_type")]
    pub project_type: Vec<String>,

    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_min_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<i64>,
}

impl ProjectMetadata {
    pub fn new(
        name: String,
        author: String,
        project_type: Option<Vec<String>>,
        cover_image: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            schema_version: 2,
            project_id: uuid::Uuid::new_v4().to_string(),
            name,
            author,
            description: None,
            cover_image,
            project_type: normalize_genres(project_type.unwrap_or_default()),
            created_at: now,
            updated_at: now,
            app_min_version: None,
            last_opened_at: Some(now),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FileNode {
    #[serde(rename = "dir")]
    Dir {
        name: String,
        path: String,
        children: Vec<FileNode>,
        created_at: i64,
        updated_at: i64,
    },
    #[serde(rename = "chapter")]
    Chapter {
        name: String,
        path: String,
        chapter_id: String,
        title: String,
        text_length_no_whitespace: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        word_count: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        created_at: i64,
        updated_at: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub project: ProjectMetadata,
    pub path: String,
    pub tree: Vec<FileNode>,
}
