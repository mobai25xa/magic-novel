use serde::{Deserialize, Serialize};

use crate::services::global_config::WorkerDefinition;

pub const DEFAULT_WORKER_MAX_ROUNDS: u32 = 20;
pub const DEFAULT_WORKER_MAX_TOOL_CALLS: u32 = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerProfile {
    pub name: String,
    pub display_name: String,
    pub system_prompt: String,
    pub tool_whitelist: Vec<String>,
    pub max_rounds: u32,
    pub max_tool_calls: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl WorkerProfile {
    pub fn from_definition(def: &WorkerDefinition) -> Self {
        Self {
            name: def.name.trim().to_string(),
            display_name: def.display_name.trim().to_string(),
            system_prompt: def.system_prompt.clone(),
            tool_whitelist: normalize_tool_whitelist(&def.tool_whitelist),
            max_rounds: def.max_rounds.unwrap_or(DEFAULT_WORKER_MAX_ROUNDS).max(1),
            max_tool_calls: def
                .max_tool_calls
                .unwrap_or(DEFAULT_WORKER_MAX_TOOL_CALLS)
                .max(1),
            model: def
                .model
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerProfileSummary {
    pub name: String,
    pub display_name: String,
    pub tool_whitelist: Vec<String>,
    pub max_rounds: u32,
    pub max_tool_calls: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub system_prompt_hash: String,
    pub profile_hash: String,
}

impl WorkerProfileSummary {
    pub fn from_profile(profile: &WorkerProfile) -> Self {
        let system_prompt_hash = hash_fnv64(&profile.system_prompt);
        let profile_hash = serde_json::to_string(profile)
            .map(|json| hash_fnv64(&json))
            .unwrap_or_else(|_| system_prompt_hash.clone());

        Self {
            name: profile.name.clone(),
            display_name: profile.display_name.clone(),
            tool_whitelist: normalize_tool_whitelist(&profile.tool_whitelist),
            max_rounds: profile.max_rounds,
            max_tool_calls: profile.max_tool_calls,
            model: profile.model.clone(),
            system_prompt_hash,
            profile_hash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRunEntry {
    pub schema_version: i32,
    pub ts: i64,
    pub mission_id: String,
    pub feature_id: String,
    pub worker_id: String,
    pub attempt: u32,
    pub profile: WorkerProfileSummary,
    pub provider: String,
    pub model: String,
}

pub fn builtin_general_worker_profile() -> WorkerProfile {
    WorkerProfile {
        name: "general-worker".to_string(),
        display_name: "General Worker".to_string(),
        system_prompt: "You are a mission worker. Complete the assigned feature safely and efficiently.\n\
If you must make assumptions, state them explicitly in your final summary.\n\
Prefer small, verifiable steps. If blocked, produce a concise failure summary with actionable next steps.".to_string(),
        tool_whitelist: normalize_tool_whitelist(&[
            "read".to_string(),
            "edit".to_string(),
            "create".to_string(),
            "ls".to_string(),
            "grep".to_string(),
            "todowrite".to_string(),
        ]),
        max_rounds: DEFAULT_WORKER_MAX_ROUNDS,
        max_tool_calls: DEFAULT_WORKER_MAX_TOOL_CALLS,
        model: None,
    }
}

pub fn builtin_integrator_worker_profile() -> WorkerProfile {
    WorkerProfile {
        name: "integrator".to_string(),
        display_name: "Integrator".to_string(),
        system_prompt: "You are an integrator worker. Your job is to read mission artifacts and produce a final handoff summary.\n\
Summarize what completed successfully, what failed, and what remains actionable.\n\
Do not modify project files unless explicitly required.".to_string(),
        tool_whitelist: normalize_tool_whitelist(&[
            "read".to_string(),
            "ls".to_string(),
            "grep".to_string(),
            "todowrite".to_string(),
        ]),
        max_rounds: 10,
        max_tool_calls: 30,
        model: None,
    }
}

fn normalize_tool_whitelist(raw: &[String]) -> Vec<String> {
    let mut out = raw
        .iter()
        .map(|t| t.trim().to_ascii_lowercase())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn hash_fnv64(text: &str) -> String {
    const OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;

    let mut hash = OFFSET_BASIS;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    format!("fnv64:{:016x}", hash)
}
