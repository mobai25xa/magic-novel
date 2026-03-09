/**
 * session-replay-tests.ts
 *
 * Dev5 (Persistence/QA Owner) — Unit tests for session replay logic.
 *
 * Pure-function tests for:
 * - replaySessionState (session-replay.ts)
 * - toSessionMessageEvent / toSessionToolResultEvent / toSessionTurnStateEvent (session-event-builders.ts)
 * - reduceSessionEventsToStore (session-reducer.ts)
 *
 * Run: npx tsx src/lib/agent-chat/session/__tests__/session-replay-tests.ts
 */

/// <reference types="node" />

import {
  toSessionMessageEvent,
  toSessionToolResultEvent,
  toSessionTurnFinalEvent,
  toSessionTurnStateEvent,
  toSessionCompactionStartedEvent,
  toSessionCompactionFinishedEvent,
  replaySessionState,
} from '../session-events'
import {
  inferHistoricalLastTurn,
  normalizeSessionHydration,
} from '../session-hydration'
import { buildTimelineEventsByTurn } from '../session-reducer-helpers'
import { createInitialSessionStorePatch } from '../store/session-store-runtime'
import {
  createApplySessionEventsAction,
  createApplySessionHydrationAction,
} from '../store/session-store-action-builders'
import type { AgentSessionEvent } from '../session-types'
import type { ChatToolTrace, ChatUiMessage } from '../../types'

// ── Assertion helpers ────────────────────────────────────────────

let passed = 0
let failed = 0

function assert(condition: boolean, message: string) {
  if (condition) {
    passed++
  } else {
    failed++
    console.error(`  FAIL: ${message}`)
  }
}

function assertEqual<T>(actual: T, expected: T, message: string) {
  const ok = actual === expected
  if (ok) {
    passed++
  } else {
    failed++
    console.error(`  FAIL: ${message} — expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`)
  }
}

// ── Test: empty events ──────────────────────────────────────────

function testEmptyEvents() {
  console.log('test: empty events → empty replay state')

  const result = replaySessionState({
    sessionId: 'test_empty',
    events: [],
  })

  assertEqual(result.sessionId, 'test_empty', 'sessionId matches')
  assertEqual(result.turn, 0, 'turn is 0')
  assertEqual(result.messages.length, 0, 'no messages')
  assertEqual(result.traces.length, 0, 'no traces')
  assert(result.lastStopReason === undefined, 'no stop reason')
  assert(result.lastTurnState === undefined, 'no turn state')
}

// ── Test: message round-trip ────────────────────────────────────

function testMessageRoundTrip() {
  console.log('test: message round-trip')

  const userMessage: ChatUiMessage = {
    id: 'msg_user_1',
    role: 'user',
    content: 'Help me edit chapter 1',
    turn: 1,
    ts: 1000,
  }

  const assistantMessage: ChatUiMessage = {
    id: 'msg_asst_1',
    role: 'assistant',
    content: 'Sure, I can help with that.',
    turn: 1,
    ts: 2000,
  }

  const events: AgentSessionEvent[] = [
    toSessionMessageEvent({ sessionId: 'test_msg', message: userMessage }),
    toSessionMessageEvent({ sessionId: 'test_msg', message: assistantMessage }),
    toSessionTurnFinalEvent({ sessionId: 'test_msg', turnId: 1, stopReason: 'success', ts: 3000 }),
  ]

  const result = replaySessionState({
    sessionId: 'test_msg',
    events,
  })

  assertEqual(result.messages.length, 2, 'two messages')
  assertEqual(result.messages[0].role, 'user', 'first is user')
  assertEqual(result.messages[0].content, 'Help me edit chapter 1', 'user content')
  assertEqual(result.messages[1].role, 'assistant', 'second is assistant')
  assertEqual(result.messages[1].content, 'Sure, I can help with that.', 'assistant content')
  assertEqual(result.turn, 1, 'turn is 1')
  assertEqual(result.lastStopReason, 'success', 'stop reason is success')
}

// ── Test: tool trace round-trip ─────────────────────────────────

function testToolTraceRoundTrip() {
  console.log('test: tool trace round-trip')

  const trace: ChatToolTrace = {
    turn: 1,
    call_id: 'tool_abc',
    tool_name: 'read',
    status: 'ok',
    duration_ms: 42,
    stage: 'result',
  }

  const events: AgentSessionEvent[] = [
    toSessionToolResultEvent({ sessionId: 'test_trace', turnId: 1, trace, ts: 1000 }),
  ]

  const result = replaySessionState({
    sessionId: 'test_trace',
    events,
  })

  assertEqual(result.traces.length, 1, 'one trace')
  assertEqual(result.traces[0].call_id, 'tool_abc', 'call_id matches')
  assertEqual(result.traces[0].tool_name, 'read', 'tool_name matches')
  assertEqual(result.traces[0].status, 'ok', 'status matches')
}

function testToolTraceRoundTripFromNestedResultEnvelope() {
  console.log('test: tool trace round-trip from trace v2 envelope')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'tool_result',
      session_id: 'test_trace_nested',
      ts: 1000,
      turn: 1,
      payload: {
        call_id: 'tool_nested_1',
        tool_name: 'edit',
        status: 'error',
        trace: {
          schema_version: 2,
          stage: 'result',
          meta: {
            tool: 'edit',
            call_id: 'tool_nested_1',
            duration_ms: 77,
          },
          result: {
            ok: false,
            preview: {},
            error: {
              code: 'E_POLICY_PLAN_REQUIRED',
              fault_domain: 'policy',
              message: 'plan required',
            },
          },
        },
      },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_trace_nested',
    events,
  })

  assertEqual(result.traces.length, 1, 'one nested trace')
  assertEqual(result.traces[0].call_id, 'tool_nested_1', 'nested call_id matches')
  assertEqual(result.traces[0].tool_name, 'edit', 'nested tool_name matches')
  assertEqual(result.traces[0].status, 'error', 'nested status matches')
  assertEqual(result.traces[0].duration_ms, 77, 'nested duration_ms matches')
  assertEqual(result.traces[0].error_code, 'E_POLICY_PLAN_REQUIRED', 'nested error_code matches')
  assertEqual(result.traces[0].fault_domain, 'policy', 'nested fault_domain matches')
}

// ── Test: turn_state round-trip ─────────────────────────────────

function testTurnStateRoundTrip() {
  console.log('test: turn_state round-trip')

  const events: AgentSessionEvent[] = [
    toSessionTurnStateEvent({
      sessionId: 'test_ts',
      turnId: 1,
      state: 'waiting_confirmation',
      payload: { call_id: 'tool_42', tool_name: 'edit' },
      ts: 1000,
    }),
  ]

  const result = replaySessionState({
    sessionId: 'test_ts',
    events,
  })

  assert(result.lastTurnState !== undefined, 'lastTurnState is set')
  assertEqual(result.lastTurnState?.turn, 1, 'turn matches')
  assertEqual(result.lastTurnState?.state, 'waiting_confirmation', 'state matches')
}

// ── Test: compaction events ─────────────────────────────────────

function testCompactionEvents() {
  console.log('test: compaction events build correctly')

  const started = toSessionCompactionStartedEvent({
    sessionId: 'test_compact',
    turnId: 1,
    reason: 'threshold',
    ts: 1000,
  })

  const finished = toSessionCompactionFinishedEvent({
    sessionId: 'test_compact',
    turnId: 1,
    meta: { removed_count: 10 },
    ts: 2000,
  })

  assertEqual(started.type, 'compaction_started', 'started type')
  assertEqual(finished.type, 'compaction_finished', 'finished type')
  assertEqual((started.payload as Record<string, unknown>)?.reason, 'threshold', 'started reason')
}

function testTodoStateReplayFromToolResultTrace() {
  console.log('test: tool_result(todowrite) trace restores todo state')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'tool_result',
      session_id: 'test_todo_trace',
      ts: 1000,
      turn: 2,
      payload: {
        call_id: 'tool_todo_1',
        tool_name: 'todowrite',
        status: 'ok',
        trace: {
          schema_version: 2,
          stage: 'result',
          meta: {
            tool: 'todowrite',
            call_id: 'tool_todo_1',
            duration_ms: 12,
          },
          result: {
            ok: true,
            preview: {
              todo_state: {
                items: [
                  { status: 'in_progress', text: 'Implement replay parser' },
                  { status: 'completed', text: 'Add live mapping' },
                ],
                last_updated_at: 123456,
                source_call_id: 'tool_todo_1',
              },
            },
            error: null,
          },
          refs: {},
        },
      },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_todo_trace',
    events,
  })

  assertEqual(result.todoState.items.length, 2, 'todo items restored from tool_result trace')
  assertEqual(result.todoState.items[0].status, 'in_progress', 'todo item status restored')
  assertEqual(result.todoState.items[0].text, 'Implement replay parser', 'todo item text restored')
  assertEqual(result.todoState.lastUpdatedAt, 123456, 'todo lastUpdatedAt restored')
  assertEqual(result.todoState.sourceCallId, 'tool_todo_1', 'todo sourceCallId restored')
  assertEqual(result.traces.length, 0, 'todowrite should not be replayed as tool trace')
}

function testTodoStateReplayIgnoresLegacyTodoWriteEvent() {
  console.log('test: legacy kind=todo_write payload is ignored after migration cleanup')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'turn_state',
      session_id: 'test_todo_legacy',
      ts: 1000,
      turn: 1,
      payload: {
        kind: 'todo_write',
        items: [
          { status: 'completed', text: 'Legacy todo event' },
        ],
        last_updated_at: 777,
        source_call_id: 'legacy_call_1',
      },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_todo_legacy',
    events,
  })

  assertEqual(result.todoState.items.length, 0, 'legacy todo payload should not mutate todo state')
  assertEqual(result.todoState.lastUpdatedAt, 0, 'legacy todo payload keeps default timestamp')
  assertEqual(result.todoState.sourceCallId, undefined, 'legacy todo payload keeps sourceCallId empty')
  assertEqual(result.traces.length, 0, 'legacy todo_write should not create traces')
}

function testTodoStateReplayIgnoresInvalidToolResultTodoState() {
  console.log('test: invalid tool_result todo_state is ignored safely')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'tool_result',
      session_id: 'test_todo_invalid',
      ts: 1000,
      turn: 1,
      payload: {
        call_id: 'tool_todo_invalid',
        tool_name: 'todowrite',
        status: 'ok',
        trace: {
          schema_version: 2,
          stage: 'result',
          meta: {
            tool: 'todowrite',
            call_id: 'tool_todo_invalid',
            duration_ms: 1,
          },
          result: {
            ok: true,
            preview: {
              todo_state: {
                items: [
                  { status: 'pending', text: '' },
                ],
                last_updated_at: 999,
              },
            },
            error: null,
          },
        },
      },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_todo_invalid',
    events,
  })

  assertEqual(result.todoState.items.length, 0, 'invalid todo_state should not overwrite state')
  assertEqual(result.todoState.lastUpdatedAt, 0, 'invalid todo_state keeps default timestamp')
}

function testCompactionFallbackReplay() {
  console.log('test: compaction fallback maps to timeline event')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'compaction_fallback',
      session_id: 'test_fallback',
      ts: 1000,
      turn: 1,
      payload: { reason: 'missing_credentials', message: 'fallback' },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_fallback',
    events,
  })

  assertEqual(result.turn, 1, 'turn advanced by fallback event')
}

function testToolExposureMetaReplay() {
  console.log('test: tool exposure telemetry meta is preserved on replay timeline')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'turn_started',
      session_id: 'test_tool_exposure',
      ts: 1000,
      turn: 3,
      payload: {
        timeline_type: 'PLAN_STARTED',
        tool_package: 'writing',
        route_reason: 'writing_signal',
        fallback_from: 'light_chat',
        fallback_reason: 'light_chat_to_writing',
        exposed_tools: ['read', 'edit', 'askuser'],
      },
    },
    {
      schema_version: 1,
      type: 'turn_completed',
      session_id: 'test_tool_exposure',
      ts: 1100,
      turn: 3,
      payload: {
        stop_reason: 'success',
        tool_package: 'writing',
        route_reason: 'writing_signal',
        exposed_tools: ['read', 'edit', 'askuser'],
      },
    },
  ]

  const grouped = buildTimelineEventsByTurn(events)
  const planEvent = grouped[3]?.find((event) => event.type === 'PLAN_STARTED')
  const completedEvent = grouped[3]?.find((event) => event.type === 'TURN_COMPLETED')

  assert(planEvent !== undefined, 'plan_started timeline event restored')
  assertEqual(planEvent?.meta?.tool_package as string | undefined, 'writing', 'plan_started tool_package preserved')
  assertEqual(completedEvent?.meta?.route_reason as string | undefined, 'writing_signal', 'turn_completed route_reason preserved')
}

// ── Test: sort ordering ─────────────────────────────────────────

function testSortOrdering() {
  console.log('test: events with out-of-order event_seq get sorted correctly')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'turn_completed',
      session_id: 'test_sort',
      ts: 3000,
      event_seq: 3,
      turn: 1,
      payload: { stop_reason: 'success' },
    },
    {
      schema_version: 1,
      type: 'message',
      session_id: 'test_sort',
      ts: 1000,
      event_seq: 1,
      turn: 1,
      payload: { role: 'user', content: 'Hello', message_id: 'msg_1' },
    },
    {
      schema_version: 1,
      type: 'message',
      session_id: 'test_sort',
      ts: 2000,
      event_seq: 2,
      turn: 1,
      payload: { role: 'assistant', content: 'Hi', message_id: 'msg_2' },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_sort',
    events,
  })

  // Events should be processed in seq order, so messages should be user first, assistant second
  assertEqual(result.messages.length, 2, 'two messages')
  assertEqual(result.messages[0].role, 'user', 'first message is user (seq=1)')
  assertEqual(result.messages[1].role, 'assistant', 'second message is assistant (seq=2)')
  assertEqual(result.lastStopReason, 'success', 'stop reason from seq=3')
}

function testReplayFallsBackToFileOrderWhenEventSeqRegresses() {
  console.log('test: replay falls back to file order when event_seq regresses across resume')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'turn_state',
      session_id: 'test_sort_regression',
      ts: 1000,
      event_seq: 2,
      turn: 4,
      payload: {
        state: 'waiting_askuser',
        call_id: 'ask_loop_1',
        tool_name: 'askuser',
        questionnaire: '1. [question] Continue?\n[topic] Flow\n[option] Yes\n[option] No',
      },
    },
    {
      schema_version: 1,
      type: 'turn_state',
      session_id: 'test_sort_regression',
      ts: 2000,
      event_seq: 1,
      turn: 4,
      payload: {
        state: 'resumed',
        call_id: 'ask_loop_1',
      },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_sort_regression',
    events,
  })

  assertEqual(result.pendingAskUser, undefined, 'regressed event_seq should not resurrect answered askuser')
  assertEqual(result.lastTurnState?.state, 'resumed', 'file-order fallback should preserve resumed as latest state')
}

// ── Test: meta overrides replay ─────────────────────────────────

function testMetaOverrides() {
  console.log('test: meta overrides replay state')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'message',
      session_id: 'test_meta',
      ts: 1000,
      turn: 1,
      payload: { role: 'user', content: 'Hi', message_id: 'msg_1' },
    },
  ]

  const result = replaySessionState({
    sessionId: 'test_meta',
    events,
    meta: {
      schema_version: 1,
      session_id: 'test_meta',
      created_at: 0,
      updated_at: 0,
      last_turn: 5,
      last_stop_reason: 'error',
      active_chapter_path: '/vol1/ch3.json',
    },
  })

  assertEqual(result.turn, 5, 'turn overridden by meta.last_turn')
  assertEqual(result.lastStopReason, 'error', 'stop reason overridden by meta')
  assertEqual(result.activeChapterPath, '/vol1/ch3.json', 'active chapter from meta')
}

function testSuspendedReplayStateIsHistoricalOnly() {
  console.log('test: replay keeps suspended state as historical only')

  const events: AgentSessionEvent[] = [
    {
      schema_version: 1,
      type: 'turn_state',
      session_id: 'test_suspended_history',
      ts: 1000,
      turn: 3,
      payload: {
        state: 'waiting_askuser',
        call_id: 'ask_1',
        tool_name: 'askuser',
        questionnaire: '1. [question] Continue?\n[topic] Flow\n[option] Yes\n[option] No',
      },
    },
  ]

  const replay = replaySessionState({
    sessionId: 'test_suspended_history',
    events,
  })

  assertEqual(replay.lastTurnState?.state, 'waiting_askuser', 'lastTurnState persisted')
  assertEqual(replay.lastTurnState?.turn, 3, 'lastTurnState turn persisted')
  assertEqual(replay.turnStopReasonById[3], 'cancel', 'historical stop reason mapped for rendering')
  assertEqual(replay.pendingAskUser?.callId, 'ask_1', 'pending askuser call restored')
  assertEqual(replay.pendingAskUser?.turn, 3, 'pending askuser turn restored')
  assertEqual(replay.pendingAskUser?.questions.length, 1, 'pending askuser questions restored')
}

function testApplySessionEventsRestoresPendingAskUser() {
  console.log('test: applySessionEvents restores pending askuser payload')

  let state = createInitialSessionStorePatch({ sessionId: 'test_apply_pending_askuser' })
  const set = (next: Partial<typeof state>) => {
    state = {
      ...state,
      ...next,
    }
  }

  const applySessionEvents = createApplySessionEventsAction(set)

  applySessionEvents({
    sessionId: 'test_apply_pending_askuser',
    events: [
      {
        schema_version: 1,
        type: 'turn_state',
        session_id: 'test_apply_pending_askuser',
        ts: 2000,
        turn: 2,
        payload: {
          state: 'waiting_askuser',
          call_id: 'ask_resume_1',
          tool_name: 'askuser',
          questions: [
            {
              question: 'Choose one',
              topic: 'Resume',
              options: ['A', 'B'],
            },
          ],
        },
      },
    ],
  })

  assertEqual(state.pendingAskUser?.callId, 'ask_resume_1', 'applySessionEvents stores pending askuser')
  assertEqual(state.pendingAskUser?.questions[0]?.topic, 'Resume', 'applySessionEvents restores askuser questions')
}

function testApplySessionHydrationReadonlyFallback() {
  console.log('test: applySessionHydration readonly fallback')

  let state = createInitialSessionStorePatch({ sessionId: 'test_readonly' })
  const set = (next: Partial<typeof state>) => {
    state = {
      ...state,
      ...next,
    }
  }

  const applyHydration = createApplySessionHydrationAction(set)

  applyHydration({
    sessionId: 'test_readonly',
    hydrationStatus: 'readonly_fallback',
    runtimeState: 'degraded',
    canContinue: false,
    canResume: false,
    readonlyReason: 'runtime_state_unavailable',
    warnings: ['runtime_snapshot_missing_and_event_rebuild_unavailable'],
  })

  assertEqual(state.session_id, 'test_readonly', 'hydrate action updates active session id')
  assertEqual(state.sessionHydrationStatus, 'readonly_fallback', 'readonly hydration status persisted')
  assertEqual(state.sessionRuntimeState, 'degraded', 'readonly runtime state persisted')
  assertEqual(state.sessionCanContinue, false, 'readonly cannot continue')
  assertEqual(state.sessionCanResume, false, 'readonly cannot resume')
  assertEqual(state.sessionReadonlyReason, 'runtime_state_unavailable', 'readonly reason persisted')
}

function testApplySessionHydrationSuspendedConfirmation() {
  console.log('test: applySessionHydration suspended confirmation capability')

  let state = createInitialSessionStorePatch({ sessionId: 'test_suspended_capability' })
  const set = (next: Partial<typeof state>) => {
    state = {
      ...state,
      ...next,
    }
  }

  const applyHydration = createApplySessionHydrationAction(set)

  applyHydration({
    sessionId: 'test_suspended_capability',
    hydrationStatus: 'snapshot_loaded',
    runtimeState: 'suspended_confirmation',
    canContinue: false,
    canResume: true,
    warnings: [],
  })

  assertEqual(state.session_id, 'test_suspended_capability', 'hydrate action updates active session id')
  assertEqual(state.sessionHydrationStatus, 'snapshot_loaded', 'suspended hydration status persisted')
  assertEqual(state.sessionRuntimeState, 'suspended_confirmation', 'suspended runtime state persisted')
  assertEqual(state.sessionCanContinue, false, 'suspended cannot continue')
  assertEqual(state.sessionCanResume, true, 'suspended can resume')
}

function testHydrationAuthorityInfersNextTurnIdFromLastTurn() {
  console.log('test: hydration authority infers nextTurnId from lastTurn')

  const hydration = normalizeSessionHydration({
    sessionId: 'test_authority',
    hydrationStatus: 'snapshot_loaded',
    runtimeState: 'ready',
    canContinue: true,
    canResume: false,
    warnings: [],
    lastTurn: 4,
    sessionRevision: 7,
  })

  assertEqual(hydration.lastTurn, 4, 'lastTurn preserved')
  assertEqual(hydration.nextTurnId, 5, 'nextTurnId inferred from lastTurn')
  assertEqual(hydration.sessionRevision, 7, 'sessionRevision preserved')
  assertEqual(hydration.hydrationSource, 'snapshot_loaded', 'hydration source defaults to hydration status')
}

function testHydrationAuthorityReadonlyFallbackDoesNotInventNextTurnId() {
  console.log('test: readonly hydration does not invent nextTurnId')

  const hydration = normalizeSessionHydration({
    sessionId: 'test_authority_readonly',
    hydrationStatus: 'readonly_fallback',
    runtimeState: 'degraded',
    canContinue: false,
    canResume: false,
    readonlyReason: 'runtime_state_unavailable' as const,
    warnings: ['runtime_snapshot_missing_and_event_rebuild_unavailable'],
    lastTurn: 3,
  })

  assertEqual(hydration.lastTurn, 3, 'readonly lastTurn preserved')
  assertEqual(hydration.nextTurnId, undefined, 'readonly nextTurnId remains undefined')
}

function testInferHistoricalLastTurnFromEventsWithoutMeta() {
  console.log('test: inferHistoricalLastTurn uses event history when meta is missing')

  const lastTurn = inferHistoricalLastTurn({
    events: [
      {
        schema_version: 1,
        type: 'message',
        session_id: 'test_last_turn',
        ts: 1000,
        turn: 6,
        payload: { role: 'assistant', content: 'done' },
      },
    ],
  })

  assertEqual(lastTurn, 6, 'last turn inferred from events')
}

function testReplayAndHydrationAuthorityRemainSeparate() {
  console.log('test: replay turn and hydration authority remain separate')

  let state = createInitialSessionStorePatch({ sessionId: 'test_replay_vs_authority' })
  const set = (next: Partial<typeof state>) => {
    state = {
      ...state,
      ...next,
    }
  }

  const applyEvents = createApplySessionEventsAction(set)
  const applyHydration = createApplySessionHydrationAction(set)

  applyEvents({
    sessionId: 'test_replay_vs_authority',
    events: [
      {
        schema_version: 1,
        type: 'message',
        session_id: 'test_replay_vs_authority',
        ts: 1000,
        turn: 4,
        payload: { role: 'user', content: 'continue', message_id: 'msg_turn_4' },
      },
    ],
  })

  applyHydration({
    sessionId: 'test_replay_vs_authority',
    hydrationStatus: 'snapshot_loaded',
    runtimeState: 'ready',
    canContinue: true,
    canResume: false,
    warnings: [],
    lastTurn: 4,
    nextTurnId: 5,
    sessionRevision: 11,
    hydrationSource: 'snapshot_loaded',
  })

  assertEqual(state.turn, 4, 'history replay still controls display turn')
  assertEqual(state.sessionReplayTurn, 4, 'sessionReplayTurn records historical replay turn')
  assertEqual(state.sessionLastTurn, 4, 'hydration lastTurn recorded separately')
  assertEqual(state.sessionNextTurnId, 5, 'hydration nextTurnId recorded separately')
  assertEqual(state.sessionRevision, 11, 'hydration revision recorded separately')
}

function testSessionEventDiagnosticsPersistBoundTurnId() {
  console.log('test: session event diagnostics include bound turn id and client request id')

  const event = toSessionMessageEvent({
    sessionId: 'test_event_diag',
    message: {
      id: 'msg_diag_1',
      role: 'user',
      content: 'hello',
      turn: 2,
      ts: 1000,
    },
    diagnostics: {
      client_request_id: 'req_diag_1',
      hydrate_source: 'snapshot_loaded',
    },
  })

  const payload = event.payload as Record<string, unknown>
  assertEqual(payload.bound_turn_id, 2, 'bound_turn_id persisted')
  assertEqual(payload.client_request_id, 'req_diag_1', 'client_request_id persisted')
  assertEqual(payload.hydrate_source, 'snapshot_loaded', 'hydrate_source persisted')
}

// ── Runner ──────────────────────────────────────────────────────

function runAll() {
  console.log('=== session-replay tests ===\n')

  testEmptyEvents()
  testMessageRoundTrip()
  testToolTraceRoundTrip()
  testToolTraceRoundTripFromNestedResultEnvelope()
  testTurnStateRoundTrip()
  testCompactionEvents()
  testTodoStateReplayFromToolResultTrace()
  testTodoStateReplayIgnoresLegacyTodoWriteEvent()
  testTodoStateReplayIgnoresInvalidToolResultTodoState()
  testCompactionFallbackReplay()
  testToolExposureMetaReplay()
  testSortOrdering()
  testReplayFallsBackToFileOrderWhenEventSeqRegresses()
  testMetaOverrides()
  testSuspendedReplayStateIsHistoricalOnly()
  testApplySessionEventsRestoresPendingAskUser()
  testApplySessionHydrationReadonlyFallback()
  testApplySessionHydrationSuspendedConfirmation()
  testHydrationAuthorityInfersNextTurnIdFromLastTurn()
  testHydrationAuthorityReadonlyFallbackDoesNotInventNextTurnId()
  testInferHistoricalLastTurnFromEventsWithoutMeta()
  testReplayAndHydrationAuthorityRemainSeparate()
  testSessionEventDiagnosticsPersistBoundTurnId()

  console.log(`\n=== Results: ${passed} passed, ${failed} failed ===`)

  if (failed > 0) {
    throw new Error(`session-replay-tests failed with ${failed} assertion(s)`)
  }
}

// Export for programmatic use
export {
  testEmptyEvents,
  testMessageRoundTrip,
  testToolTraceRoundTrip,
  testToolTraceRoundTripFromNestedResultEnvelope,
  testTurnStateRoundTrip,
  testCompactionEvents,
  testTodoStateReplayFromToolResultTrace,
  testTodoStateReplayIgnoresLegacyTodoWriteEvent,
  testTodoStateReplayIgnoresInvalidToolResultTodoState,
  testCompactionFallbackReplay,
  testToolExposureMetaReplay,
  testSortOrdering,
  testReplayFallsBackToFileOrderWhenEventSeqRegresses,
  testMetaOverrides,
  testSuspendedReplayStateIsHistoricalOnly,
  testApplySessionEventsRestoresPendingAskUser,
  testApplySessionHydrationReadonlyFallback,
  testApplySessionHydrationSuspendedConfirmation,
  testHydrationAuthorityInfersNextTurnIdFromLastTurn,
  testHydrationAuthorityReadonlyFallbackDoesNotInventNextTurnId,
  testInferHistoricalLastTurnFromEventsWithoutMeta,
  testReplayAndHydrationAuthorityRemainSeparate,
  testSessionEventDiagnosticsPersistBoundTurnId,
  runAll,
}

// Run if executed directly
if (typeof process !== 'undefined' && process.argv[1]?.includes('session-replay-tests')) {
  runAll()
}
