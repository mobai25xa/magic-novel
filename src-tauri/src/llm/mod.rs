//! LLM Layer - Multi-provider streaming + unified accumulator
//!
//! Based on docs/magic_plan/plan_agent_parallel/02-dev2-llm-owner.md

pub mod accumulator;
pub mod bootstrap;
pub mod constants;
pub mod errors;
pub mod inspiration;
pub mod provider;
pub mod providers;
pub mod router;
pub mod router_factory;
pub mod streaming_turn;
pub mod types;
