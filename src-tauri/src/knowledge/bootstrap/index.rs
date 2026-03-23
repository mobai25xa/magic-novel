use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::models::AppError;
use crate::utils::atomic_write::atomic_write_json;

use super::writeback::BootstrapWrittenArtifact;

const OBJECT_INDEX_PATH: &str = ".magic_novel/index/object_index.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BootstrapObjectIndex {
    #[serde(default = "default_schema_version")]
    schema_version: i32,
    #[serde(default)]
    updated_at: i64,
    #[serde(default)]
    objects: Vec<BootstrapObjectIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BootstrapObjectIndexEntry {
    kind: String,
    path: String,
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    source_job_id: String,
    updated_at: i64,
}

fn default_schema_version() -> i32 {
    1
}

pub fn upsert_index_entries(
    project_path: &Path,
    artifacts: &[BootstrapWrittenArtifact],
    source_job_id: &str,
) -> Result<(), AppError> {
    let path = project_path.join(OBJECT_INDEX_PATH);
    let mut doc = if path.exists() {
        let raw = std::fs::read_to_string(&path)?;
        serde_json::from_str::<BootstrapObjectIndex>(&raw)?
    } else {
        BootstrapObjectIndex {
            schema_version: 1,
            updated_at: 0,
            objects: Vec::new(),
        }
    };

    for artifact in artifacts {
        let entry = BootstrapObjectIndexEntry {
            kind: artifact.kind.as_str().to_string(),
            path: artifact.path.clone(),
            status: artifact.status.clone(),
            title: artifact.title.clone(),
            summary: artifact.summary.clone(),
            source_job_id: source_job_id.to_string(),
            updated_at: artifact.updated_at,
        };

        if let Some(existing) = doc
            .objects
            .iter_mut()
            .find(|item| item.path == artifact.path)
        {
            *existing = entry;
        } else {
            doc.objects.push(entry);
        }
    }

    doc.updated_at = chrono::Utc::now().timestamp_millis();
    doc.objects
        .sort_by(|left, right| left.path.cmp(&right.path));
    atomic_write_json(&path, &doc)
}
