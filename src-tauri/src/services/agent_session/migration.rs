use std::path::Path;

use crate::models::AppError;

use super::stream::read_events_jsonl;
use super::types::AgentSessionEvent;
use super::AGENT_SESSION_SCHEMA_VERSION;

pub fn migrate_event(event: &mut AgentSessionEvent) {
    match event.schema_version {
        0 => {
            event.schema_version = AGENT_SESSION_SCHEMA_VERSION;
        }
        AGENT_SESSION_SCHEMA_VERSION => {}
        _ => {
            // Future schema versions: keep as-is for forward-compatible reads.
        }
    }
}

pub fn read_and_migrate(path: &Path) -> Result<Vec<AgentSessionEvent>, AppError> {
    let mut events = read_events_jsonl(path)?;
    for event in &mut events {
        migrate_event(event);
    }
    Ok(events)
}
