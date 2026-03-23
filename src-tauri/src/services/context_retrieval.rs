use std::path::PathBuf;

use crate::models::{Chapter, VolumeMetadata};
use crate::services::{list_dirs, list_files, read_json};

const MANUSCRIPTS_DIR: &str = "manuscripts";

#[derive(Debug, Clone)]
pub struct OutlineDataset {
    pub volumes: Vec<OutlineVolumeEntry>,
}

#[derive(Debug, Clone)]
pub struct OutlineVolumeEntry {
    pub title: String,
    pub volume_path: String,
    pub chapter_count: u32,
    pub word_count: u64,
    pub chapters: Vec<OutlineChapterEntry>,
}

#[derive(Debug, Clone)]
pub struct OutlineChapterEntry {
    pub title: String,
    pub chapter_path: String,
    pub word_count: u64,
    pub status: Option<String>,
}

pub fn load_outline_dataset(project_path: &str, volume_filter: Option<&str>) -> OutlineDataset {
    let manuscripts_root = PathBuf::from(project_path).join(MANUSCRIPTS_DIR);
    let volume_dirs = list_dirs(&manuscripts_root).unwrap_or_default();

    let mut volumes = Vec::new();
    for vol_dir in volume_dirs {
        if let Some(filter) = volume_filter {
            if vol_dir != filter {
                continue;
            }
        }

        let vol_path = manuscripts_root.join(&vol_dir);
        let vol_meta: VolumeMetadata = match read_json(&vol_path.join("volume.json")) {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        let chapter_files = list_files(&vol_path, ".json").unwrap_or_default();
        let mut chapter_count = 0_u32;
        let mut word_count = 0_u64;
        let mut chapters = Vec::new();

        for ch_file in chapter_files {
            if ch_file == "volume.json" {
                continue;
            }

            let chapter: Chapter = match read_json(&vol_path.join(&ch_file)) {
                Ok(chapter) => chapter,
                Err(_) => continue,
            };

            chapter_count += 1;
            let chapter_words = chapter.counts.text_length_no_whitespace.max(0) as u64;
            word_count += chapter_words;

            chapters.push(OutlineChapterEntry {
                title: chapter.title,
                chapter_path: format!("{}/{}", vol_dir, ch_file),
                word_count: chapter_words,
                status: chapter
                    .status
                    .as_ref()
                    .map(|status| format!("{:?}", status)),
            });
        }

        volumes.push(OutlineVolumeEntry {
            title: vol_meta.title,
            volume_path: vol_dir,
            chapter_count,
            word_count,
            chapters,
        });
    }

    OutlineDataset { volumes }
}
