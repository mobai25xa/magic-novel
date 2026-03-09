use crate::models::{AiProposal, AppError};
use crate::services::ensure_dir;
use crate::utils::atomic_write::atomic_write_json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tauri::command;

const MAGIC_NOVEL_DIR: &str = "magic_novel";
const AI_DIR: &str = "ai";
const PROPOSALS_DIR: &str = "proposals";
const HISTORY_DIR: &str = "history";
const CHAPTERS_DIR: &str = "chapters";

#[command]
pub async fn save_ai_proposal(
    project_path: String,
    proposal: AiProposal,
) -> Result<String, AppError> {
    let project_path = PathBuf::from(&project_path);
    let proposals_dir = project_path
        .join(MAGIC_NOVEL_DIR)
        .join(AI_DIR)
        .join(PROPOSALS_DIR);

    ensure_dir(&proposals_dir)?;

    let proposal_file = proposals_dir.join(format!("{}.json", proposal.proposal_id));
    atomic_write_json(&proposal_file, &proposal)?;

    Ok(proposal.proposal_id.clone())
}

#[command]
pub async fn get_ai_proposal(
    project_path: String,
    proposal_id: String,
) -> Result<AiProposal, AppError> {
    let project_path = PathBuf::from(&project_path);
    let proposal_file = project_path
        .join(MAGIC_NOVEL_DIR)
        .join(AI_DIR)
        .join(PROPOSALS_DIR)
        .join(format!("{}.json", proposal_id));

    if !proposal_file.exists() {
        return Err(AppError::not_found("Proposal 不存在"));
    }

    let content = fs::read_to_string(&proposal_file)?;
    let proposal: AiProposal = serde_json::from_str(&content)?;
    Ok(proposal)
}

#[command]
pub async fn update_proposal_status(
    project_path: String,
    proposal_id: String,
    status: String,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let proposal_file = project_path
        .join(MAGIC_NOVEL_DIR)
        .join(AI_DIR)
        .join(PROPOSALS_DIR)
        .join(format!("{}.json", proposal_id));

    if !proposal_file.exists() {
        return Err(AppError::not_found("Proposal 不存在"));
    }

    let content = fs::read_to_string(&proposal_file)?;
    let mut proposal: AiProposal = serde_json::from_str(&content)?;

    proposal.status = match status.as_str() {
        "generated" => crate::models::ProposalStatus::Generated,
        "accepted" => crate::models::ProposalStatus::Accepted,
        "partially_accepted" => crate::models::ProposalStatus::PartiallyAccepted,
        "rejected" => crate::models::ProposalStatus::Rejected,
        _ => return Err(AppError::invalid_argument("无效的状态")),
    };

    atomic_write_json(&proposal_file, &proposal)?;

    Ok(())
}

#[command]
pub async fn append_chapter_history_event(
    project_path: String,
    chapter_id: String,
    event: serde_json::Value,
) -> Result<(), AppError> {
    let project_path = PathBuf::from(&project_path);
    let history_dir = project_path
        .join(MAGIC_NOVEL_DIR)
        .join(HISTORY_DIR)
        .join(CHAPTERS_DIR);

    ensure_dir(&history_dir)?;

    let history_file = history_dir.join(format!("{}.jsonl", chapter_id));

    let event_line = serde_json::to_string(&event)? + "\n";

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_file)?;

    file.write_all(event_line.as_bytes())?;

    Ok(())
}

#[command]
pub async fn get_chapter_history(
    project_path: String,
    chapter_id: String,
) -> Result<Vec<serde_json::Value>, AppError> {
    let project_path = PathBuf::from(&project_path);
    let history_file = project_path
        .join(MAGIC_NOVEL_DIR)
        .join(HISTORY_DIR)
        .join(CHAPTERS_DIR)
        .join(format!("{}.jsonl", chapter_id));

    if !history_file.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&history_file)?;
    let events: Vec<serde_json::Value> = content
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    Ok(events)
}

#[command]
pub async fn list_ai_proposals(
    project_path: String,
    chapter_id: Option<String>,
) -> Result<Vec<AiProposal>, AppError> {
    let project_path = PathBuf::from(&project_path);
    let proposals_dir = project_path
        .join(MAGIC_NOVEL_DIR)
        .join(AI_DIR)
        .join(PROPOSALS_DIR);

    if !proposals_dir.exists() {
        return Ok(vec![]);
    }

    let mut proposals = Vec::new();

    for entry in fs::read_dir(&proposals_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let content = fs::read_to_string(&path)?;
            if let Ok(proposal) = serde_json::from_str::<AiProposal>(&content) {
                if let Some(ref filter_id) = chapter_id {
                    if &proposal.chapter_id == filter_id {
                        proposals.push(proposal);
                    }
                } else {
                    proposals.push(proposal);
                }
            }
        }
    }

    proposals.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(proposals)
}
