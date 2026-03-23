//! Agent Engine - Per-session state manager
//!
//! Manages per-session state including:
//! - Incrementing turn_id per session
//! - CancellationToken for running turns
//! - Suspended turn state for pause/resume
//!
//! Uses a global singleton backed by DashMap for lock-free concurrent access.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio_util::sync::CancellationToken;

use super::messages::{AgentMessage, ConversationState};
use super::types::{LoopConfig, StopReason, ToolCallInfo};

fn normalize_next_turn_id(next_turn_id: u32) -> u32 {
    next_turn_id.max(1)
}

pub fn derive_next_turn_id(
    last_turn: Option<u32>,
    conversation: Option<&ConversationState>,
    persisted_next_turn_id: Option<u32>,
) -> u32 {
    let last_turn = last_turn.unwrap_or(0);
    let current_turn = conversation.map(|state| state.current_turn).unwrap_or(0);
    let persisted_turn = persisted_next_turn_id.unwrap_or(1).saturating_sub(1);

    last_turn
        .max(current_turn)
        .max(persisted_turn)
        .saturating_add(1)
        .max(1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelledTurnInfo {
    pub turn_id: u32,
    pub client_request_id: Option<String>,
}

/// State for a suspended (paused) turn awaiting resume.
#[derive(Debug, Clone)]
pub struct SuspendedTurnState {
    pub conversation_state: ConversationState,
    pub pending_tool_call: ToolCallInfo,
    pub pending_call_id: String,
    pub remaining_tool_calls: Vec<ToolCallInfo>,
    pub completed_messages: Vec<AgentMessage>,
    pub loop_config: LoopConfig,
    pub project_path: String,
    /// The turn engine info needed to resume: provider name, model, base_url, api_key
    pub provider_name: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub active_chapter_path: Option<String>,
    pub active_skill: Option<String>,
    pub system_prompt: Option<String>,
    pub suspend_reason: StopReason,
    /// Rounds already executed before suspension
    pub rounds_executed: u32,
    /// Total tool calls before suspension
    pub total_tool_calls: u32,
}

/// Per-session state entry.
struct SessionState {
    next_turn_id: AtomicU32,
    cancel_token: Option<CancellationToken>,
    active_turn_id: u32,
    active_client_request_id: Option<String>,
    suspended: Option<SuspendedTurnState>,
    conversation: Option<ConversationState>,
    last_active: Instant,
}

impl SessionState {
    fn new() -> Self {
        Self {
            next_turn_id: AtomicU32::new(1),
            cancel_token: None,
            active_turn_id: 0,
            active_client_request_id: None,
            suspended: None,
            conversation: None,
            last_active: Instant::now(),
        }
    }

    fn touch(&mut self) {
        self.last_active = Instant::now();
    }
}

/// Global session state manager.
///
/// Thread-safe, lock-free concurrent access via DashMap.
pub struct SessionStateManager {
    sessions: DashMap<String, SessionState>,
    turn_locks: DashMap<String, Arc<Mutex<()>>>,
}

impl SessionStateManager {
    fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            turn_locks: DashMap::new(),
        }
    }

    pub fn with_session_turn_lock<R>(&self, session_id: &str, f: impl FnOnce() -> R) -> R {
        let lock = self
            .turn_locks
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock().expect("session turn lock poisoned");
        f()
    }

    pub fn peek_next_turn_id(&self, session_id: &str) -> u32 {
        self.sessions
            .get(session_id)
            .map(|entry| normalize_next_turn_id(entry.next_turn_id.load(Ordering::Relaxed)))
            .unwrap_or(1)
    }

    pub fn seed_next_turn_id(&self, session_id: &str, next_turn_id: u32) {
        let mut entry = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(SessionState::new);
        entry.touch();
        entry
            .next_turn_id
            .store(normalize_next_turn_id(next_turn_id), Ordering::Relaxed);
    }

    pub fn save_runtime_state(
        &self,
        session_id: &str,
        state: ConversationState,
        next_turn_id: u32,
        suspended: Option<SuspendedTurnState>,
    ) {
        let authoritative_next_turn_id =
            derive_next_turn_id(None, Some(&state), Some(next_turn_id));

        let mut entry = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(SessionState::new);
        entry.touch();
        entry
            .next_turn_id
            .store(authoritative_next_turn_id, Ordering::Relaxed);
        entry.conversation = Some(state);
        entry.suspended = suspended;
    }

    pub fn save_suspended_runtime_state(
        &self,
        session_id: &str,
        state: SuspendedTurnState,
        next_turn_id: u32,
    ) {
        let conversation = state.conversation_state.clone();
        let authoritative_next_turn_id =
            derive_next_turn_id(None, Some(&conversation), Some(next_turn_id));

        let mut entry = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(SessionState::new);
        entry.touch();
        entry
            .next_turn_id
            .store(authoritative_next_turn_id, Ordering::Relaxed);
        entry.conversation = Some(conversation);
        entry.suspended = Some(state);
    }

    /// Get the next turn_id for a session (auto-incrementing, starts at 1).
    pub fn next_turn_id(&self, session_id: &str) -> u32 {
        let mut entry = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(SessionState::new);
        entry.touch();
        entry.next_turn_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Store a CancellationToken for the current running turn.
    pub fn set_cancel_token(
        &self,
        session_id: &str,
        turn_id: u32,
        token: CancellationToken,
        client_request_id: Option<String>,
    ) {
        let mut entry = self
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(SessionState::new);
        entry.touch();
        entry.cancel_token = Some(token);
        entry.active_turn_id = turn_id;
        entry.active_client_request_id = client_request_id;
    }

    /// Cancel the running turn for a session.
    /// Returns the cancelled turn metadata if a token was found.
    pub fn cancel_session(&self, session_id: &str) -> Option<CancelledTurnInfo> {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.touch();
            if let Some(token) = entry.cancel_token.take() {
                token.cancel();
                let cancelled = CancelledTurnInfo {
                    turn_id: entry.active_turn_id,
                    client_request_id: entry.active_client_request_id.take(),
                };
                entry.active_turn_id = 0;
                return Some(cancelled);
            }
        }
        None
    }

    /// Clear the cancel token (called when turn completes normally).
    pub fn clear_cancel_token(&self, session_id: &str) {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.touch();
            entry.cancel_token = None;
            entry.active_turn_id = 0;
            entry.active_client_request_id = None;
        }
    }

    /// Store suspended turn state for later resume.
    pub fn suspend_turn(&self, session_id: &str, state: SuspendedTurnState) {
        let next_turn_id = derive_next_turn_id(None, Some(&state.conversation_state), None);
        self.save_suspended_runtime_state(session_id, state, next_turn_id);
    }

    /// Save current conversation state for next turn continuity.
    pub fn save_conversation(&self, session_id: &str, state: ConversationState) {
        let next_turn_id = derive_next_turn_id(None, Some(&state), None);
        self.save_runtime_state(session_id, state, next_turn_id, None);
    }

    /// Take (remove) saved conversation state for next turn.
    pub fn take_conversation(&self, session_id: &str) -> Option<ConversationState> {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.touch();
            return entry.conversation.take();
        }
        None
    }

    /// Take (remove) the suspended turn state for resume.
    pub fn take_suspended(&self, session_id: &str) -> Option<SuspendedTurnState> {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.touch();
            let suspended = entry.suspended.take();
            if suspended.is_some() {
                entry.conversation = None;
            }
            return suspended;
        }
        None
    }

    /// Remove a session entirely (cleanup on session end).
    pub fn remove_session(&self, session_id: &str) {
        // Cancel any running turn before removing
        if let Some((_, mut state)) = self.sessions.remove(session_id) {
            if let Some(token) = state.cancel_token.take() {
                token.cancel();
            }
            state.active_turn_id = 0;
            state.active_client_request_id = None;
            state.conversation = None;
            state.suspended = None;
        }
        self.turn_locks.remove(session_id);
    }

    /// Get the current turn_id for a session (without incrementing).
    pub fn current_turn_id(&self, session_id: &str) -> u32 {
        self.sessions
            .get(session_id)
            .map(|e| e.next_turn_id.load(Ordering::Relaxed).saturating_sub(1))
            .unwrap_or(0)
    }

    /// Check if a session has an active running turn.
    pub fn has_active_turn(&self, session_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .map(|e| e.cancel_token.is_some())
            .unwrap_or(false)
    }

    /// Check if a session has a suspended turn.
    pub fn has_suspended(&self, session_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .map(|e| e.suspended.is_some())
            .unwrap_or(false)
    }

    /// Clean up inactive sessions older than the provided TTL.
    ///
    /// Sessions with active cancel token or suspended turn are retained.
    pub fn cleanup_stale(&self, ttl: Duration) {
        let now = Instant::now();
        let stale_keys: Vec<String> = self
            .sessions
            .iter()
            .filter_map(|entry| {
                let state = entry.value();
                let is_active = state.cancel_token.is_some() || state.suspended.is_some();
                if is_active || now.duration_since(state.last_active) <= ttl {
                    None
                } else {
                    Some(entry.key().clone())
                }
            })
            .collect();

        for key in &stale_keys {
            self.sessions.remove(key);
            self.turn_locks.remove(key);
        }

        if !stale_keys.is_empty() {
            tracing::info!(
                target: "agent_engine",
                cleaned = stale_keys.len(),
                "cleaned up stale sessions"
            );
        }
    }

    #[cfg(test)]
    fn set_last_active_for_test(&self, session_id: &str, last_active: Instant) {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.last_active = last_active;
        }
    }
}

/// Global singleton accessor.
pub fn global() -> &'static SessionStateManager {
    static INSTANCE: std::sync::OnceLock<SessionStateManager> = std::sync::OnceLock::new();
    INSTANCE.get_or_init(SessionStateManager::new)
}

#[cfg(test)]
mod tests {
    use super::super::types::StopReason;
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[test]
    fn test_turn_id_incrementing() {
        let mgr = SessionStateManager::new();
        assert_eq!(mgr.next_turn_id("s1"), 1);
        assert_eq!(mgr.next_turn_id("s1"), 2);
        assert_eq!(mgr.next_turn_id("s1"), 3);
        // Different session starts at 1
        assert_eq!(mgr.next_turn_id("s2"), 1);
    }

    #[test]
    fn test_current_turn_id() {
        let mgr = SessionStateManager::new();
        assert_eq!(mgr.current_turn_id("s1"), 0); // no turns yet
        mgr.next_turn_id("s1");
        assert_eq!(mgr.current_turn_id("s1"), 1);
        mgr.next_turn_id("s1");
        assert_eq!(mgr.current_turn_id("s1"), 2);
    }

    #[test]
    fn test_cancel_token() {
        let mgr = SessionStateManager::new();
        let token = CancellationToken::new();
        let child = token.clone();

        mgr.set_cancel_token("s1", 3, token, Some("req_1".to_string()));
        assert!(!child.is_cancelled());

        let cancelled = mgr.cancel_session("s1").expect("cancelled turn metadata");
        assert_eq!(cancelled.turn_id, 3);
        assert_eq!(cancelled.client_request_id.as_deref(), Some("req_1"));
        assert!(child.is_cancelled());

        // Second cancel returns none (token was taken)
        assert!(mgr.cancel_session("s1").is_none());
    }

    #[test]
    fn test_cancel_nonexistent_session() {
        let mgr = SessionStateManager::new();
        assert!(mgr.cancel_session("nonexistent").is_none());
    }

    #[test]
    fn test_remove_session_cancels_token() {
        let mgr = SessionStateManager::new();
        let token = CancellationToken::new();
        let child = token.clone();

        mgr.set_cancel_token("s1", 1, token, None);
        mgr.next_turn_id("s1"); // create some state

        mgr.remove_session("s1");
        assert!(child.is_cancelled());
        // Session state is gone
        assert_eq!(mgr.current_turn_id("s1"), 0);
    }

    #[test]
    fn test_clear_cancel_token() {
        let mgr = SessionStateManager::new();
        let token = CancellationToken::new();
        let child = token.clone();

        mgr.set_cancel_token("s1", 4, token, Some("req_clear".to_string()));
        mgr.clear_cancel_token("s1");

        // Token was cleared, not cancelled
        assert!(!child.is_cancelled());
        assert!(mgr.cancel_session("s1").is_none());
    }

    #[test]
    fn test_has_active_turn() {
        let mgr = SessionStateManager::new();
        assert!(!mgr.has_active_turn("s1"));

        mgr.set_cancel_token("s1", 1, CancellationToken::new(), None);
        assert!(mgr.has_active_turn("s1"));

        mgr.clear_cancel_token("s1");
        assert!(!mgr.has_active_turn("s1"));
    }

    #[test]
    fn test_has_suspended() {
        let mgr = SessionStateManager::new();
        assert!(!mgr.has_suspended("s1"));
        // We can't easily construct a full SuspendedTurnState in tests here
        // but the API is verified
    }

    #[test]
    fn test_save_and_take_conversation_roundtrip() {
        let mgr = SessionStateManager::new();
        let mut state = ConversationState::new("s1".to_string());
        state.messages.push(AgentMessage::user("hello".to_string()));

        mgr.save_conversation("s1", state.clone());

        let restored = mgr.take_conversation("s1");
        assert!(restored.is_some());
        let restored = restored.unwrap();
        assert_eq!(restored.session_id, "s1");
        assert_eq!(restored.messages.len(), 1);

        // take removes it
        assert!(mgr.take_conversation("s1").is_none());
    }

    #[test]
    fn test_take_conversation_missing_returns_none() {
        let mgr = SessionStateManager::new();
        assert!(mgr.take_conversation("missing").is_none());
    }

    #[test]
    fn test_cleanup_stale_removes_inactive_session() {
        let mgr = SessionStateManager::new();
        mgr.next_turn_id("stale");
        mgr.set_last_active_for_test("stale", Instant::now() - Duration::from_secs(3600));

        mgr.cleanup_stale(Duration::from_secs(300));

        assert_eq!(mgr.current_turn_id("stale"), 0);
    }

    #[test]
    fn test_cleanup_stale_keeps_active_cancel_token() {
        let mgr = SessionStateManager::new();
        mgr.set_cancel_token("active", 1, CancellationToken::new(), None);
        mgr.set_last_active_for_test("active", Instant::now() - Duration::from_secs(3600));

        mgr.cleanup_stale(Duration::from_secs(300));

        assert!(mgr.cancel_session("active").is_some());
    }

    #[test]
    fn test_cleanup_stale_keeps_suspended_session() {
        let mgr = SessionStateManager::new();
        let suspended = SuspendedTurnState {
            conversation_state: ConversationState::new("s1".to_string()),
            pending_tool_call: ToolCallInfo {
                llm_call_id: "call_1".to_string(),
                tool_name: "draft_write".to_string(),
                args: serde_json::json!({}),
            },
            pending_call_id: "pending_1".to_string(),
            remaining_tool_calls: Vec::new(),
            completed_messages: Vec::new(),
            loop_config: LoopConfig::default(),
            project_path: "D:/tmp/project".to_string(),
            provider_name: "openai-compatible".to_string(),
            model: "gpt-4o-mini".to_string(),
            base_url: "https://example.com".to_string(),
            api_key: "key".to_string(),
            active_chapter_path: None,
            active_skill: None,
            system_prompt: None,
            suspend_reason: StopReason::WaitingConfirmation,
            rounds_executed: 0,
            total_tool_calls: 0,
        };
        mgr.suspend_turn("paused", suspended);
        mgr.set_last_active_for_test("paused", Instant::now() - Duration::from_secs(3600));

        mgr.cleanup_stale(Duration::from_secs(300));

        assert!(mgr.has_suspended("paused"));
    }

    #[test]
    fn test_global_singleton() {
        let g1 = global();
        let g2 = global();
        assert!(std::ptr::eq(g1, g2));
    }

    #[test]
    fn test_seed_next_turn_id_overrides_cursor() {
        let mgr = SessionStateManager::new();
        mgr.next_turn_id("s1");
        mgr.seed_next_turn_id("s1", 8);

        assert_eq!(mgr.peek_next_turn_id("s1"), 8);
        assert_eq!(mgr.next_turn_id("s1"), 8);
        assert_eq!(mgr.peek_next_turn_id("s1"), 9);
    }

    #[test]
    fn test_save_runtime_state_keeps_cursor_in_sync() {
        let mgr = SessionStateManager::new();
        let mut state = ConversationState::new("s1".to_string());
        state.current_turn = 7;
        state.messages.push(AgentMessage::user("hello".to_string()));

        mgr.save_runtime_state("s1", state, 8, None);

        assert_eq!(mgr.peek_next_turn_id("s1"), 8);
        assert_eq!(mgr.current_turn_id("s1"), 7);
    }

    #[test]
    fn test_suspend_turn_keeps_cursor_in_sync() {
        let mgr = SessionStateManager::new();
        let mut conversation = ConversationState::new("s1".to_string());
        conversation.current_turn = 5;

        mgr.suspend_turn(
            "s1",
            SuspendedTurnState {
                conversation_state: conversation,
                pending_tool_call: ToolCallInfo {
                    llm_call_id: "call_1".to_string(),
                    tool_name: "draft_write".to_string(),
                    args: serde_json::json!({}),
                },
                pending_call_id: "pending_1".to_string(),
                remaining_tool_calls: Vec::new(),
                completed_messages: Vec::new(),
                loop_config: LoopConfig::default(),
                project_path: "D:/tmp/project".to_string(),
                provider_name: "openai-compatible".to_string(),
                model: "gpt-4o-mini".to_string(),
                base_url: "https://example.com".to_string(),
                api_key: "key".to_string(),
                active_chapter_path: None,
                active_skill: None,
                system_prompt: None,
                suspend_reason: StopReason::WaitingConfirmation,
                rounds_executed: 0,
                total_tool_calls: 0,
            },
        );

        assert_eq!(mgr.peek_next_turn_id("s1"), 6);
        assert!(mgr.has_suspended("s1"));
    }

    #[test]
    fn test_with_session_turn_lock_serializes_seed_and_allocate() {
        let mgr = Arc::new(SessionStateManager::new());
        let results = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();

        mgr.seed_next_turn_id("shared", 8);

        for _ in 0..2 {
            let mgr = Arc::clone(&mgr);
            let results = Arc::clone(&results);
            handles.push(std::thread::spawn(move || {
                let turn_id = mgr.with_session_turn_lock("shared", || mgr.next_turn_id("shared"));
                results.lock().expect("lock results").push(turn_id);
            }));
        }

        for handle in handles {
            handle.join().expect("join thread");
        }

        let turns = results.lock().expect("results lock");
        let unique: HashSet<u32> = turns.iter().copied().collect();
        assert_eq!(unique.len(), 2);
        assert!(unique.contains(&8));
        assert!(unique.contains(&9));
    }
}
