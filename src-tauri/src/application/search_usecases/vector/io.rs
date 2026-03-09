use std::path::Path;

use crate::models::AppError;

use crate::application::search_usecases::index::io::write_bytes_atomic;

use super::types::VectorManifest;

pub fn read_f32_le_vec(path: &Path) -> Result<Vec<f32>, AppError> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let bytes = std::fs::read(path).map_err(AppError::from)?;
    let mut out = Vec::with_capacity(bytes.len() / 4);

    for chunk in bytes.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    Ok(out)
}

pub fn write_f32_le_vec_atomic(path: &Path, values: &[f32]) -> Result<(), AppError> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    write_bytes_atomic(path, &bytes)
}

pub fn read_vector_manifest(path: &Path) -> Result<VectorManifest, AppError> {
    crate::services::read_json(path)
}

pub fn write_vector_manifest(path: &Path, manifest: &VectorManifest) -> Result<(), AppError> {
    crate::utils::atomic_write::atomic_write_json(path, manifest)
}
