//! Agent Engine Layer - Core loop, turn engine, tool scheduler, compaction, events, persistence
//!
//! Based on docs/magic_plan/plan_agent_parallel/01-dev1-engine-owner.md

pub mod compaction;
pub mod context_loader;
pub mod delegate_runtime;
pub mod emitter;
pub mod events;
pub mod exposure_policy;
pub mod llm_compaction_summarizer;
pub mod loop_engine;
pub mod messages;
pub mod persistence;
pub mod prompt_assembler;
pub mod recovery;
pub mod reminder_builder;
#[cfg(test)]
mod reminder_builder_tests;
pub mod session_state;
pub mod skills;
pub mod text_utils;
pub mod todowrite;
pub mod tool_dispatch;
pub mod tool_errors;
pub mod tool_formatters;
pub mod tool_routing;
pub mod tool_scheduler;
pub mod tool_schemas;
pub mod turn;
pub mod types;
pub mod worker_identity;
