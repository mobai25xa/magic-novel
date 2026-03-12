/**
 * runtime-backend-events.ts
 *
 * Dev4 (UI/Event Owner) — Event subscription layer.
 *
 * Listens to Rust-emitted Tauri events (`magic:agent_event` / `magic:mission_event`)
 * and maps them to zustand store actions in `useAgentChatStore`.
 *
 * Aligned with:
 *   - docs/magic_plan/plan_agent/07-agent-event-protocol.md
 *   - docs/magic_plan/plan_agent_parallel/guide.md (contract)
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { AgentAskUserQuestion, AgentToolTrace } from '@/agent/types'
import { readChapter } from '@/features/editor-reading'
import { refreshProjectTreeLifecycle } from '@/features/project-lifecycle'
import {
  agentTurnCancelClient,
} from '@/platform/tauri/clients/agent-engine-client'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { normalizeTodoStateFromToolResultPayload } from './todo'
import {
  mapStructuredAskUserQuestions,
  parseAskUserQuestionnaire,
} from './askuser'
import {
  extractToolPreviewRefs,
  parseToolTraceV2,
  toFaultDomain,
} from './tool-trace'
import { useAgentChatStore } from './store'

// ── Event channel names (must match Rust constants) ─────────────

const AGENT_EVENT_CHANNEL = 'magic:agent_event'
const MISSION_EVENT_CHANNEL = 'magic:mission_event'
const TOOL_RESULT_REFRESH_DEBOUNCE_MS = 80

// ── Agent event envelope shape (mirrors Rust EventEnvelope) ─────

interface AgentEventEnvelope {
  schema_version: number
  event_id: string
  ts: number
  session_id: string
  turn_id: number
  client_request_id?: string
  source: {
    kind: string
    worker_id?: string
    mission_id?: string
  }
  type: string
  payload: Record<string, unknown>
}

// ── Mission event envelope shape ────────────────────────────────

interface MissionEventEnvelope {
  schema_version: number
  event_id: string
  ts: number
  mission_id: string
  type: string
  payload: Record<string, unknown>
}

type ToolChangeSet = {
  shouldRefreshTree: boolean
  shouldRefreshEditor: boolean
  chapterPath?: string
  projectPath?: string
}

let pendingToolRefreshTimer: ReturnType<typeof setTimeout> | null = null
let pendingToolChangeSet: ToolChangeSet | null = null

function mergeToolChangeSet(input: ToolChangeSet) {
  if (!pendingToolChangeSet) {
    pendingToolChangeSet = {
      shouldRefreshTree: input.shouldRefreshTree,
      shouldRefreshEditor: input.shouldRefreshEditor,
      chapterPath: input.chapterPath,
      projectPath: input.projectPath,
    }
    return
  }

  pendingToolChangeSet = {
    shouldRefreshTree: pendingToolChangeSet.shouldRefreshTree || input.shouldRefreshTree,
    shouldRefreshEditor: pendingToolChangeSet.shouldRefreshEditor || input.shouldRefreshEditor,
    chapterPath: input.chapterPath || pendingToolChangeSet.chapterPath,
    projectPath: input.projectPath || pendingToolChangeSet.projectPath,
  }
}

function extractToolChapterPath(input: {
  payload: Record<string, unknown>
  traceRefs: ReturnType<typeof extractToolPreviewRefs>
}) {
  const fromNewPath = typeof input.traceRefs?.path === 'string' ? input.traceRefs.path : undefined
  if (fromNewPath && fromNewPath.trim()) {
    return fromNewPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')
  }

  const fromArgs = asRecord(input.payload.args)
  const chapterPath = typeof fromArgs?.chapter_path === 'string'
    ? fromArgs.chapter_path
    : typeof fromArgs?.path === 'string'
      ? fromArgs.path
      : undefined

  if (chapterPath && chapterPath.trim()) {
    return chapterPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')
  }

  const activeChapterPath = useAgentChatStore.getState().active_chapter_path
  return activeChapterPath && activeChapterPath.trim()
    ? activeChapterPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')
    : undefined
}

function normalizeCreatedKind(value: unknown): 'volume' | 'chapter' | null {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'volume' || normalized === 'folder') return 'volume'
  if (normalized === 'chapter' || normalized === 'file') return 'chapter'
  return null
}

async function applyToolRefresh(changeSet: ToolChangeSet) {
  const projectPath = (changeSet.projectPath || useProjectStore.getState().projectPath || '').trim()
  if (!projectPath) {
    return
  }

  if (changeSet.shouldRefreshTree) {
    try {
      const tree = await refreshProjectTreeLifecycle(projectPath)
      useProjectStore.getState().setTree(tree)
    } catch (error) {
      console.error('[agent-event] refresh tree failed:', error)
    }
  }

  if (changeSet.shouldRefreshEditor && changeSet.chapterPath) {
    const chapterPath = changeSet.chapterPath.trim()
    const editorStore = useEditorStore.getState()
    if (!chapterPath || editorStore.currentChapterPath !== chapterPath) {
      return
    }

    try {
      const chapter = await readChapter(projectPath, chapterPath)
      useProjectStore.getState().setSelectedPath(chapterPath)
      editorStore.setCurrentChapter(chapter.id, chapterPath, chapter.title)
      editorStore.setContent(chapter.content)
      editorStore.setIsDirty(false)
      editorStore.setLastOpened(projectPath, chapterPath, chapter.id, chapter.title)
      useAgentChatStore.getState().setActiveChapterPath(chapterPath)
    } catch (error) {
      console.error('[agent-event] refresh editor failed:', error)
    }
  }
}

function scheduleToolRefresh(changeSet: ToolChangeSet) {
  mergeToolChangeSet(changeSet)

  if (pendingToolRefreshTimer) {
    return
  }

  pendingToolRefreshTimer = setTimeout(() => {
    const next = pendingToolChangeSet
    pendingToolRefreshTimer = null
    pendingToolChangeSet = null

    if (!next) {
      return
    }

    void applyToolRefresh(next)
  }, TOOL_RESULT_REFRESH_DEBOUNCE_MS)
}

function buildToolRefreshChangeSet(input: {
  toolName: string
  status: string
  payload: Record<string, unknown>
  tracePreview: Record<string, unknown>
  traceRefs: ReturnType<typeof extractToolPreviewRefs>
}): ToolChangeSet | null {
  if (input.status !== 'ok') {
    return null
  }

  const toolName = input.toolName
  if (toolName !== 'create' && toolName !== 'edit' && toolName !== 'delete' && toolName !== 'move') {
    return null
  }

  const chapterPath = extractToolChapterPath({
    payload: input.payload,
    traceRefs: input.traceRefs,
  })
  const createdKind = normalizeCreatedKind(input.tracePreview.created_kind)
  const shouldRefreshEditor = toolName === 'edit'
    || (toolName === 'create' && createdKind === 'chapter' && Boolean(chapterPath))
    || (toolName === 'move' && Boolean(chapterPath))

  return {
    shouldRefreshTree: true,
    shouldRefreshEditor,
    chapterPath,
  }
}

function extractClientRequestId(envelope: AgentEventEnvelope) {
  if (typeof envelope.client_request_id === 'string' && envelope.client_request_id.trim()) {
    return envelope.client_request_id
  }

  if (typeof envelope.payload.client_request_id === 'string' && envelope.payload.client_request_id.trim()) {
    return envelope.payload.client_request_id
  }

  return undefined
}

function getOldestPendingClientRequestId() {
  const pendingRequests = Object.values(useAgentChatStore.getState().pendingRequestsByClientRequestId)
  if (pendingRequests.length === 0) {
    return undefined
  }

  return [...pendingRequests]
    .sort((left, right) => left.createdAt - right.createdAt)
    .at(0)?.clientRequestId
}

function bindPendingRequestFromEnvelope(envelope: AgentEventEnvelope) {
  const store = useAgentChatStore.getState()
  const clientRequestId = extractClientRequestId(envelope)
    ?? store.clientRequestIdByTurnId[envelope.turn_id]
    ?? getOldestPendingClientRequestId()

  if (!clientRequestId) {
    return
  }

  let result: ReturnType<typeof store.bindPendingTurnRequest> | null = null
  try {
    result = store.bindPendingTurnRequest({
      clientRequestId,
      turn: envelope.turn_id,
    })
  } catch (error) {
    console.warn('[agent-event] bind pending request failed:', error)
    return
  }

  if (result?.cancelRequested) {
    agentTurnCancelClient({
      session_id: envelope.session_id,
      turn_id: envelope.turn_id,
    }).catch((error) => {
      console.error('[agent-event] cancel-after-bind failed:', error)
    })
  }
}

function shouldIgnoreAskUserRequest(input: {
  store: ReturnType<typeof useAgentChatStore.getState>
  turn: number
  callId: string
}) {
  const pending = input.store.pendingAskUser
  if (pending?.turn === input.turn && pending.callId === input.callId) {
    return true
  }

  const currentStep = input.store.stepsByTurnId[input.turn]?.find((step) => step.callId === input.callId)
  return currentStep?.progress === 'answered' || currentStep?.status === 'success'
}

function isWorkerAgentEnvelope(envelope: AgentEventEnvelope) {
  if (envelope.source?.kind !== 'worker') {
    return false
  }

  const workerId = typeof envelope.source.worker_id === 'string' ? envelope.source.worker_id.trim() : ''
  const missionId = typeof envelope.source.mission_id === 'string' ? envelope.source.mission_id.trim() : ''
  return Boolean(workerId && missionId)
}

// ── Agent event dispatcher ──────────────────────────────────────

function dispatchAgentEvent(envelope: AgentEventEnvelope) {
  if (isWorkerAgentEnvelope(envelope)) {
    dispatchWorkerAgentEvent(envelope)
    return
  }

  const store = useAgentChatStore.getState()
  if (envelope.session_id !== store.session_id) {
    return
  }

  bindPendingRequestFromEnvelope(envelope)

  const turn = envelope.turn_id
  const ts = envelope.ts
  const payload = envelope.payload

  switch (envelope.type) {
    case 'TURN_STARTED': {
      store.markTurnStarted(turn)
      store.setStateStatus('thinking')
      store.setSessionRuntimeCapability({
        runtimeState: 'running',
        canContinue: false,
        canResume: false,
        readonlyReason: undefined,
      })
      store.pushTurnEvent(turn, {
        type: 'TURN_STARTED',
        ts,
        meta: extractToolExposureMeta(payload),
        summary: `turn started · ${String(payload.model_provider || '')}/${String(payload.model || '')}`,
      })
      break
    }

    case 'PLAN_STARTED': {
      store.setTurnPhase(turn, 'planning')
      store.pushTurnEvent(turn, {
        type: 'PLAN_STARTED',
        ts,
        meta: extractToolExposureMeta(payload),
        summary: buildToolExposureSummary(payload),
      })
      break
    }

    case 'STREAMING_STARTED': {
      store.setTurnPhase(turn, 'planning')
      store.pushTurnEvent(turn, {
        type: 'STREAMING_STARTED',
        ts,
        summary: 'streaming started',
      })
      break
    }

    case 'ASSISTANT_TEXT_DELTA': {
      const delta = String(payload.delta ?? '')
      if (delta) {
        store.appendTurnAnswerDelta(turn, delta)
      }
      break
    }

    case 'THINKING_TEXT_DELTA': {
      const delta = String(payload.delta ?? '')
      if (delta) {
        store.appendTurnThinkingDelta(turn, delta)
      }
      break
    }

    case 'TOOL_CALL_STARTED': {
      const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
      const toolName = String(payload.tool_name ?? '')
      const normalizedToolName = toolName.trim().toLowerCase()

      if (normalizedToolName === 'todowrite') {
        break
      }

      store.setStateStatus('executing_tool')
      store.markToolStepStarted(turn, {
        callId,
        llmCallId: payload.llm_call_id as string | undefined,
        toolName,
        args: parseArgsPreview(payload.args_preview),
        ts,
      })
      store.pushTurnEvent(turn, {
        type: 'TOOL_CALL_STARTED',
        ts,
        callId,
        summary: `${toolName} · started`,
      })
      break
    }

    case 'TOOL_CALL_PROGRESS': {
      const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
      const toolName = String(payload.tool_name ?? '')
      const normalizedToolName = toolName.trim().toLowerCase()

      if (normalizedToolName === 'todowrite') {
        break
      }

      store.markToolStepProgress(turn, {
        callId,
        llmCallId: payload.llm_call_id as string | undefined,
        toolName,
        progress: String(payload.progress ?? 'running'),
        ts,
      })
      break
    }

    case 'TOOL_CALL_FINISHED': {
      const parsedTrace = parseToolTraceV2(payload.trace)
      const callId = String(parsedTrace?.meta.call_id ?? payload.call_id ?? payload.llm_call_id ?? '')
      const toolName = String(parsedTrace?.meta.tool ?? payload.tool_name ?? '')
      const normalizedToolName = toolName.trim().toLowerCase()
      const status = parsedTrace ? (parsedTrace.result.ok ? 'ok' : 'error') : String(payload.status ?? 'ok')
      const traceError = parsedTrace?.result.error || null
      const traceFaultDomain = toFaultDomain(traceError?.fault_domain)
      const traceStage = parsedTrace?.stage || toToolTraceStage(payload.stage)
      const tracePreview = parsedTrace?.result.preview || {}
      const traceRefs = extractToolPreviewRefs(payload.trace)

      if (normalizedToolName === 'todowrite') {
        if (status === 'ok' && typeof store.applyTodoState === 'function') {
          const todoState = normalizeTodoStateFromToolResultPayload(payload)
          if (todoState) {
            store.applyTodoState(todoState)
          }
        }
        break
      }

      const completedTrace: AgentToolTrace = {
        turn,
        call_id: callId,
        tool_name: toolName,
        status: status === 'ok' ? 'ok' : 'error',
        duration_ms: parsedTrace?.meta.duration_ms
          ?? (typeof payload.duration_ms === 'number' ? payload.duration_ms : 0),
        fault_domain: traceFaultDomain,
        error_code: typeof traceError?.code === 'string' ? traceError.code : undefined,
        error_message: typeof traceError?.message === 'string'
          ? traceError.message
          : undefined,
        stage: traceStage,
        revision_before: parsedTrace?.meta.revision_before,
        revision_after: parsedTrace?.meta.revision_after,
        tx_id: parsedTrace?.meta.tx_id,
        preview: Object.keys(tracePreview).length > 0 ? tracePreview : undefined,
        refs: traceRefs || undefined,
      }

      store.markToolStepCompleted(turn, {
        callId,
        llmCallId: payload.llm_call_id as string | undefined,
        toolName,
        output: JSON.stringify(parsedTrace ?? payload.trace ?? {}),
        trace: completedTrace,
        ts,
      })
      store.pushTurnEvent(turn, {
        type: 'TOOL_CALL_FINISHED',
        ts,
        callId,
        summary: `${toolName} · ${status}`,
      })

      const refreshChangeSet = buildToolRefreshChangeSet({
        toolName,
        status,
        payload,
        tracePreview,
        traceRefs,
      })
      if (refreshChangeSet) {
        scheduleToolRefresh(refreshChangeSet)
      }

      break
    }

    case 'WAITING_FOR_CONFIRMATION': {
      const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
      const toolName = String(payload.tool_name ?? '')
      store.setStateStatus('waiting_confirmation')
      // Only unlock resume after TURN_COMPLETED confirms the loop is fully suspended.
      store.markWaitingForConfirmation(turn, {
        callId,
        llmCallId: payload.llm_call_id as string | undefined,
        toolName,
        waitState: 'waiting_confirmation',
        ts,
      })
      store.pushTurnEvent(turn, {
        type: 'WAITING_FOR_CONFIRMATION',
        ts,
        callId,
        summary: `${toolName} · waiting confirmation: ${String(payload.reason ?? '')}`,
      })
      break
    }

    case 'ASKUSER_REQUESTED': {
      const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
      if (!callId) break
      if (shouldIgnoreAskUserRequest({ store, turn, callId })) break

      // Parse canonical structured questions first; questionnaire is display-only fallback.
      let questions: AgentAskUserQuestion[] | null = null
      let questionnaire = ''

      if (Array.isArray(payload.questions) && payload.questions.length > 0) {
        questions = mapStructuredAskUserQuestions(payload.questions)
        questionnaire = questions
          ? questions.map((q, i) => `${i + 1}. ${q.question}`).join('\n')
          : ''
      }

      if (!questions && typeof payload.questionnaire === 'string' && payload.questionnaire) {
        const parsed = parseAskUserQuestionnaire(payload.questionnaire)
        if (parsed.ok) {
          questions = parsed.questions
          questionnaire = parsed.questionnaire
        }
      }

      // If parsing failed, degrade gracefully to a single fallback question and keep interaction in chat.
      if (!questions || questions.length === 0) {
        console.warn('[agent-event] ASKUSER_REQUESTED: failed to parse questions, using fallback question')
        questions = [{
          index: 0,
          question: '无法解析问题格式。你希望我如何继续？',
          topic: 'askuser_fallback',
          options: ['继续执行', '取消本次操作'],
        }]
        questionnaire = '1. 无法解析问题格式。你希望我如何继续？'
      }

      store.setStateStatus('waiting_askuser')
      // Only unlock resume after TURN_COMPLETED confirms the loop is fully suspended.
      store.markWaitingForConfirmation(turn, {
        callId,
        llmCallId: payload.llm_call_id as string | undefined,
        toolName: String(payload.tool_name ?? 'askuser'),
        waitState: 'waiting_askuser',
        ts,
      })
      store.openAskUserRequest({
        callId,
        turn,
        questionnaire,
        questions,
        requestedAt: ts,
      })

      store.pushTurnEvent(turn, {
        type: 'ASKUSER_REQUESTED',
        ts,
        callId,
        summary: 'askuser · waiting user',
      })
      break
    }

    case 'ASKUSER_ANSWERED': {
      store.clearPendingAskUser()
      store.setStateStatus('thinking')
      store.setSessionRuntimeCapability({
        runtimeState: 'running',
        canContinue: false,
        canResume: false,
        readonlyReason: undefined,
      })
      store.setTurnPhase(turn, 'synthesizing')
      store.pushTurnEvent(turn, {
        type: 'ASKUSER_ANSWERED',
        ts,
        callId: String(payload.call_id ?? payload.llm_call_id ?? ''),
        summary: 'askuser · answered',
      })
      break
    }

    case 'USAGE_UPDATE': {
      // Usage tracking — currently just push as timeline event for observability
      store.pushTurnEvent(turn, {
        type: 'TURN_STARTED', // reuse; no dedicated usage event type
        ts,
        meta: payload,
        summary: `tokens: in=${String(payload.input_tokens ?? 0)} out=${String(payload.output_tokens ?? 0)}`,
      })
      break
    }

    case 'COMPACTION_STARTED': {
      store.setStateStatus('compacting')
      store.setTurnPhase(turn, 'compacting')
      store.pushTurnEvent(turn, {
        type: 'COMPACTION_STARTED',
        ts,
        summary: `compaction started: ${String(payload.reason ?? '')}`,
      })
      break
    }

    case 'COMPACTION_FINISHED': {
      store.pushTurnEvent(turn, {
        type: 'COMPACTION_FINISHED',
        ts,
        summary: 'compaction finished',
        meta: payload.meta as Record<string, unknown> | undefined,
      })
      break
    }

    case 'COMPACTION_FALLBACK': {
      store.pushTurnEvent(turn, {
        type: 'COMPACTION_FALLBACK',
        ts,
        summary: String(payload.message ?? payload.reason ?? 'compaction fallback'),
        meta: payload,
      })
      break
    }

    case 'TURN_COMPLETED': {
      const rawStopReason = String(payload.stop_reason ?? 'success')
      if (rawStopReason === 'waiting_confirmation') {
        store.setStateStatus('waiting_confirmation')
        store.setSessionRuntimeCapability({
          runtimeState: 'suspended_confirmation',
          canContinue: false,
          canResume: true,
          readonlyReason: undefined,
        })
      } else if (rawStopReason === 'waiting_askuser') {
        store.setStateStatus('waiting_askuser')
        store.setSessionRuntimeCapability({
          runtimeState: 'suspended_askuser',
          canContinue: false,
          canResume: true,
          readonlyReason: undefined,
        })
      } else {
        store.setStateStatus('idle')
        store.setSessionRuntimeCapability({
          runtimeState: rawStopReason === 'cancel' ? 'cancelled' : 'ready',
          canContinue: true,
          canResume: false,
          readonlyReason: undefined,
        })
      }

      const phase = rawStopReason === 'waiting_confirmation' || rawStopReason === 'waiting_askuser'
        ? 'tool_running'
        : rawStopReason === 'cancel'
          ? 'cancelled'
          : 'completed'

      store.setTurnPhase(turn, phase, {
        stopReason: normalizeStopReason(payload.stop_reason),
        finishedAt: ts,
      })
      if (typeof payload.latency_ms === 'number') {
        store.setLastTurnLatency(payload.latency_ms)
      }
      store.pushTurnEvent(turn, {
        type: 'TURN_COMPLETED',
        ts,
        meta: extractToolExposureMeta(payload),
        summary: `turn completed · ${rawStopReason}`,
      })
      if (rawStopReason !== 'waiting_confirmation' && rawStopReason !== 'waiting_askuser') {
        store.commitTurnTimelineSnapshot(turn)
      }
      break
    }

    case 'TURN_FAILED': {
      store.setStateStatus('idle')
      const errorMsg = String(payload.error_message ?? payload.error_code ?? 'unknown error')
      const errorCode = String(payload.error_code ?? 'E_LLM_UNKNOWN')
      const errorDetail = asRecord(payload.error_detail)

      if (errorCode === 'E_CANCELLED') {
        store.setSessionRuntimeCapability({
          runtimeState: 'cancelled',
          canContinue: true,
          canResume: false,
          readonlyReason: undefined,
        })
        store.setTurnPhase(turn, 'cancelled', {
          stopReason: 'cancel',
          finishedAt: ts,
        })
        store.pushTurnEvent(turn, {
          type: 'TURN_CANCELLED',
          ts,
          summary: 'turn cancelled',
        })
        store.commitTurnTimelineSnapshot(turn)
        break
      }

      store.setSessionRuntimeCapability({
        runtimeState: 'failed',
        canContinue: true,
        canResume: false,
        readonlyReason: undefined,
      })

      store.setTurnPhase(turn, 'failed', {
        error: errorMsg,
        turnError: {
          code: errorCode,
          message: errorMsg,
          detail: errorDetail ? {
            provider: typeof errorDetail.provider === 'string' ? errorDetail.provider : undefined,
            model: typeof errorDetail.model === 'string' ? errorDetail.model : undefined,
            retryable: typeof errorDetail.retryable === 'boolean' ? errorDetail.retryable : undefined,
            diagnostic: typeof errorDetail.diagnostic === 'string' ? errorDetail.diagnostic : undefined,
            http_status: typeof errorDetail.http_status === 'number' ? errorDetail.http_status : undefined,
            retry_after_ms: typeof errorDetail.retry_after_ms === 'number' ? errorDetail.retry_after_ms : undefined,
            category_hint: typeof errorDetail.category_hint === 'string' ? errorDetail.category_hint : undefined,
            tool_name: typeof errorDetail.tool_name === 'string' ? errorDetail.tool_name : undefined,
            schema_path: typeof errorDetail.schema_path === 'string' ? errorDetail.schema_path : undefined,
            tool_package: typeof errorDetail.tool_package === 'string' ? errorDetail.tool_package : undefined,
            route_reason: typeof errorDetail.route_reason === 'string' ? errorDetail.route_reason : undefined,
            fallback_from: typeof errorDetail.fallback_from === 'string' ? errorDetail.fallback_from : undefined,
            fallback_reason: typeof errorDetail.fallback_reason === 'string' ? errorDetail.fallback_reason : undefined,
            rollout_mode: typeof errorDetail.rollout_mode === 'string' ? errorDetail.rollout_mode : undefined,
            rollout_in_canary: typeof errorDetail.rollout_in_canary === 'boolean'
              ? errorDetail.rollout_in_canary
              : undefined,
            canary_percent: typeof errorDetail.canary_percent === 'number' ? errorDetail.canary_percent : undefined,
            turn_failed_classification: typeof errorDetail.turn_failed_classification === 'string'
              ? errorDetail.turn_failed_classification
              : undefined,
            provider_schema_error: typeof errorDetail.provider_schema_error === 'boolean'
              ? errorDetail.provider_schema_error
              : undefined,
            provider_400_error: typeof errorDetail.provider_400_error === 'boolean'
              ? errorDetail.provider_400_error
              : undefined,
            missing_tool_escalation: typeof errorDetail.missing_tool_escalation === 'boolean'
              ? errorDetail.missing_tool_escalation
              : undefined,
            tool_call_count: typeof errorDetail.tool_call_count === 'number' ? errorDetail.tool_call_count : undefined,
            rounds_executed: typeof errorDetail.rounds_executed === 'number' ? errorDetail.rounds_executed : undefined,
            fallback_occurred: typeof errorDetail.fallback_occurred === 'boolean'
              ? errorDetail.fallback_occurred
              : undefined,
            exposed_tools: Array.isArray(errorDetail.exposed_tools)
              ? errorDetail.exposed_tools.filter((value): value is string => typeof value === 'string')
              : undefined,
            skipped_tools: Array.isArray(errorDetail.skipped_tools)
              ? errorDetail.skipped_tools
                .map((value) => asRecord(value))
                .filter((value): value is Record<string, unknown> => Boolean(value))
                .map((value) => ({
                  tool_name: typeof value.tool_name === 'string' ? value.tool_name : undefined,
                  error: typeof value.error === 'string' ? value.error : undefined,
                }))
              : undefined,
          } : undefined,
        },
        finishedAt: ts,
      })
      store.pushTurnEvent(turn, {
        type: 'TURN_FAILED',
        ts,
        summary: errorMsg,
        meta: { error_code: errorCode, error_detail: errorDetail },
      })
      store.commitTurnTimelineSnapshot(turn)
      break
    }

    case 'TURN_CANCELLED': {
      store.setStateStatus('idle')
      store.setSessionRuntimeCapability({
        runtimeState: 'cancelled',
        canContinue: true,
        canResume: false,
        readonlyReason: undefined,
      })
      store.setTurnPhase(turn, 'cancelled', {
        stopReason: 'cancel',
        finishedAt: ts,
      })
      store.pushTurnEvent(turn, {
        type: 'TURN_CANCELLED',
        ts,
        summary: 'turn cancelled',
      })
      store.commitTurnTimelineSnapshot(turn)
      break
    }

    default: {
      // Unknown event type — log for debugging, no-op in store
      console.warn('[agent-event] unknown event type:', envelope.type, envelope)
    }
  }
}

// ── Mission event dispatcher ────────────────────────────────────

/** Mission event state tracked in-memory (minimal UI in Phase 1) */
export interface MissionUiState {
  missionId: string
  state: string
  currentFeatureId?: string
  workerStatuses: Record<string, { featureId: string; status: string; summary?: string; updatedAt: number }>
  progressLog: Array<{ ts: number; message: string }>
  /** Optional P1: used to trigger UI refresh when Layer1 artifacts change. */
  layer1UpdatedAt?: number
  /** Optional P1: used to trigger UI refresh when ContextPack is rebuilt. */
  contextPackBuiltAt?: number
  /** Optional M3: used to trigger UI refresh when ReviewReport is recorded. */
  reviewUpdatedAt?: number
  /** Optional M3: backend indicates a decision is required to proceed. */
  reviewDecisionRequired?: boolean
  /** Optional M3: raw decision payload for UI rendering. */
  reviewDecision?: Record<string, unknown> | null
  /** Optional M4: used to trigger UI refresh when Knowledge writeback changes. */
  knowledgeUpdatedAt?: number
  /** Optional M4: backend indicates a knowledge decision is required to proceed. */
  knowledgeDecisionRequired?: boolean
  /** Optional M4: raw decision payload for UI rendering. */
  knowledgeDecision?: Record<string, unknown> | null
}

const MAX_MISSION_PROGRESS_ENTRIES = 40
const MAX_MISSION_WORKER_HISTORY = 8

let currentMissionState: MissionUiState | null = null
const missionListeners: Array<(state: MissionUiState | null) => void> = []

function isTerminalMissionState(state: string) {
  return state === 'completed' || state === 'cancelled' || state === 'failed'
}

function createMissionUiState(missionId: string): MissionUiState {
  return {
    missionId,
    state: 'unknown',
    workerStatuses: {},
    progressLog: [],
    layer1UpdatedAt: undefined,
    contextPackBuiltAt: undefined,
    reviewUpdatedAt: undefined,
    reviewDecisionRequired: undefined,
    reviewDecision: null,
    knowledgeUpdatedAt: undefined,
    knowledgeDecisionRequired: undefined,
    knowledgeDecision: null,
  }
}

function resetMissionTransientState(state: MissionUiState): MissionUiState {
  return {
    ...state,
    currentFeatureId: undefined,
    workerStatuses: {},
    progressLog: [],
    layer1UpdatedAt: undefined,
    contextPackBuiltAt: undefined,
    reviewUpdatedAt: undefined,
    reviewDecisionRequired: undefined,
    reviewDecision: null,
    knowledgeUpdatedAt: undefined,
    knowledgeDecisionRequired: undefined,
    knowledgeDecision: null,
  }
}

function pruneMissionWorkerStatuses(
  workerStatuses: MissionUiState['workerStatuses'],
): MissionUiState['workerStatuses'] {
  const entries = Object.entries(workerStatuses).sort(([, left], [, right]) => right.updatedAt - left.updatedAt)
  const running = entries.filter(([, info]) => info.status === 'running')
  const settled = entries
    .filter(([, info]) => info.status !== 'running')
    .slice(0, MAX_MISSION_WORKER_HISTORY)

  return Object.fromEntries([...running, ...settled])
}

function upsertMissionWorkerStatus(
  workerStatuses: MissionUiState['workerStatuses'],
  workerId: string,
  nextStatus: MissionUiState['workerStatuses'][string],
): MissionUiState['workerStatuses'] {
  return pruneMissionWorkerStatuses({
    ...workerStatuses,
    [workerId]: nextStatus,
  })
}

function trimMissionProgressLog(entries: MissionUiState['progressLog']): MissionUiState['progressLog'] {
  return entries.slice(-MAX_MISSION_PROGRESS_ENTRIES)
}

export function getMissionUiState(): MissionUiState | null {
  return currentMissionState
}

export function subscribeMissionUiState(
  listener: (state: MissionUiState | null) => void,
): () => void {
  missionListeners.push(listener)
  return () => {
    const idx = missionListeners.indexOf(listener)
    if (idx >= 0) missionListeners.splice(idx, 1)
  }
}

function notifyMissionListeners() {
  const snapshot = currentMissionState
  for (const fn of missionListeners) {
    fn(snapshot)
  }
}

function extractWorkerFeatureIdFromPayload(payload: Record<string, unknown>) {
  const featureId = payload.feature_id
  if (typeof featureId === 'string' && featureId.trim()) {
    return featureId.trim()
  }
  return undefined
}

function summarizeWorkerAgentEvent(envelope: AgentEventEnvelope): { status: string; summary?: string } | null {
  const payload = envelope.payload
  const turn = envelope.turn_id

  switch (envelope.type) {
    case 'TURN_STARTED': {
      const provider = String(payload.model_provider ?? '')
      const model = String(payload.model ?? '')
      const modelHint = provider || model ? `${provider}/${model}`.replace(/^\//, '').replace(/\/$/, '') : ''
      return {
        status: 'running',
        summary: modelHint ? `turn ${turn} · ${modelHint}` : `turn ${turn} · started`,
      }
    }

    case 'TOOL_CALL_STARTED': {
      const toolName = String(payload.tool_name ?? '').trim()
      if (!toolName) return null
      if (toolName.toLowerCase() === 'todowrite') return null
      return {
        status: 'running',
        summary: `${toolName} · started`,
      }
    }

    case 'TOOL_CALL_PROGRESS': {
      const toolName = String(payload.tool_name ?? '').trim()
      if (!toolName) return null
      if (toolName.toLowerCase() === 'todowrite') return null
      const progress = String(payload.progress ?? 'running')
      return {
        status: 'running',
        summary: `${toolName} · ${progress}`,
      }
    }

    case 'TOOL_CALL_FINISHED': {
      const parsedTrace = parseToolTraceV2(payload.trace)
      const toolName = String(parsedTrace?.meta.tool ?? payload.tool_name ?? '').trim()
      if (!toolName) return null
      if (toolName.toLowerCase() === 'todowrite') return null
      const ok = parsedTrace ? parsedTrace.result.ok : String(payload.status ?? 'ok') === 'ok'
      return {
        status: 'running',
        summary: `${toolName} · ${ok ? 'ok' : 'error'}`,
      }
    }

    case 'WAITING_FOR_CONFIRMATION': {
      const toolName = String(payload.tool_name ?? '').trim() || 'tool'
      return {
        status: 'running',
        summary: `${toolName} · waiting confirmation`,
      }
    }

    case 'ASKUSER_REQUESTED': {
      return {
        status: 'running',
        summary: 'askuser · waiting user',
      }
    }

    case 'TURN_COMPLETED': {
      const rawStopReason = String(payload.stop_reason ?? 'success')
      if (rawStopReason === 'cancel') {
        return { status: 'cancelled', summary: `turn ${turn} · cancelled` }
      }
      if (rawStopReason === 'error') {
        return { status: 'failed', summary: `turn ${turn} · error` }
      }
      if (rawStopReason === 'waiting_confirmation' || rawStopReason === 'waiting_askuser') {
        return { status: 'running', summary: `turn ${turn} · ${rawStopReason}` }
      }
      return {
        status: 'completed',
        summary: `turn ${turn} · ${rawStopReason}`,
      }
    }

    case 'TURN_FAILED': {
      const errorCode = String(payload.error_code ?? '')
      const errorMsg = String(payload.error_message ?? payload.error_code ?? 'unknown error')
      if (errorCode === 'E_CANCELLED') {
        return { status: 'cancelled', summary: `turn ${turn} · cancelled` }
      }
      return {
        status: 'failed',
        summary: `turn ${turn} · ${errorMsg}`,
      }
    }

    case 'TURN_CANCELLED': {
      return { status: 'cancelled', summary: `turn ${turn} · cancelled` }
    }

    default:
      return null
  }
}

function dispatchWorkerAgentEvent(envelope: AgentEventEnvelope) {
  const workerId = String(envelope.source.worker_id ?? '').trim()
  const missionId = String(envelope.source.mission_id ?? '').trim()
  if (!workerId || !missionId) {
    return
  }

  const update = summarizeWorkerAgentEvent(envelope)
  if (!update) {
    return
  }

  const base = (!currentMissionState || currentMissionState.missionId !== missionId)
    ? createMissionUiState(missionId)
    : currentMissionState

  const previous = base.workerStatuses[workerId]
  const featureId = extractWorkerFeatureIdFromPayload(envelope.payload)
    ?? previous?.featureId
    ?? ''

  currentMissionState = {
    ...base,
    workerStatuses: upsertMissionWorkerStatus(
      base.workerStatuses,
      workerId,
      {
        featureId,
        status: update.status,
        summary: update.summary,
        updatedAt: envelope.ts,
      },
    ),
  }

  notifyMissionListeners()
}

function dispatchMissionEvent(envelope: MissionEventEnvelope) {
  const payload = envelope.payload

  const base = (!currentMissionState || currentMissionState.missionId !== envelope.mission_id)
    ? createMissionUiState(envelope.mission_id)
    : currentMissionState

  let nextState: MissionUiState = base

  switch (envelope.type) {
    case 'MISSION_STATE_CHANGED': {
      const newState = String(payload.new_state ?? 'unknown')
      nextState = {
        ...base,
        state: newState,
      }

      if (isTerminalMissionState(newState)) {
        nextState = resetMissionTransientState(nextState)
      }
      break
    }

    case 'MISSION_FEATURES_CHANGED': {
      const featureId = String(payload.feature_id ?? '').trim()
      nextState = {
        ...base,
        currentFeatureId: featureId || undefined,
      }
      break
    }

    case 'MISSION_PROGRESS_ENTRY': {
      nextState = {
        ...base,
        progressLog: trimMissionProgressLog([
          ...base.progressLog,
          {
            ts: envelope.ts,
            message: String(payload.message ?? ''),
          },
        ]),
      }
      break
    }

    case 'WORKER_STARTED': {
      const workerId = String(payload.worker_id ?? '')
      const featureId = String(payload.feature_id ?? '')
      if (!workerId.trim()) {
        break
      }

      nextState = {
        ...base,
        workerStatuses: upsertMissionWorkerStatus(
          base.workerStatuses,
          workerId,
          {
            featureId,
            status: 'running',
            updatedAt: envelope.ts,
          },
        ),
      }
      break
    }

    case 'WORKER_COMPLETED': {
      const workerId = String(payload.worker_id ?? '')
      const featureId = String(payload.feature_id ?? '')
      if (!workerId.trim()) {
        break
      }

      nextState = {
        ...base,
        workerStatuses: upsertMissionWorkerStatus(
          base.workerStatuses,
          workerId,
          {
            featureId,
            status: payload.ok ? 'completed' : 'failed',
            summary: String(payload.summary ?? ''),
            updatedAt: envelope.ts,
          },
        ),
      }
      break
    }

    case 'MISSION_HEARTBEAT': {
      // No-op for now, could update last-seen timestamp
      break
    }

    case 'MISSION_LAYER1_UPDATED': {
      nextState = {
        ...base,
        layer1UpdatedAt: envelope.ts,
      }
      break
    }

    case 'MISSION_CONTEXTPACK_BUILT': {
      nextState = {
        ...base,
        contextPackBuiltAt: envelope.ts,
      }
      break
    }

    case 'MISSION_REVIEW_RECORDED': {
      nextState = {
        ...base,
        reviewUpdatedAt: envelope.ts,
        reviewDecisionRequired: false,
        reviewDecision: null,
      }
      break
    }

    case 'MISSION_REVIEW_DECISION_REQUIRED': {
      nextState = {
        ...base,
        reviewUpdatedAt: envelope.ts,
        reviewDecisionRequired: true,
        reviewDecision: payload,
      }
      break
    }

    case 'MISSION_KNOWLEDGE_PROPOSED': {
      const bundleId = typeof payload.bundle_id === 'string'
        ? payload.bundle_id.trim()
        : typeof payload.bundleId === 'string'
          ? payload.bundleId.trim()
          : ''
      const counts = asRecord(payload.counts)
      const items = typeof counts?.items === 'number'
        ? counts.items
        : typeof payload.item_count === 'number'
          ? payload.item_count
          : undefined

      nextState = {
        ...base,
        knowledgeUpdatedAt: envelope.ts,
        knowledgeDecisionRequired: false,
        knowledgeDecision: null,
        progressLog: trimMissionProgressLog([
          ...base.progressLog,
          {
            ts: envelope.ts,
            message: `Knowledge proposed${typeof items === 'number' ? ` · ${items} items` : ''}${bundleId ? ` · ${bundleId.slice(0, 10)}…` : ''}`,
          },
        ]),
      }
      break
    }

    case 'MISSION_KNOWLEDGE_DECISION_REQUIRED': {
      const deltaId = typeof payload.delta_id === 'string'
        ? payload.delta_id.trim()
        : typeof payload.deltaId === 'string'
          ? payload.deltaId.trim()
          : ''
      const conflicts = Array.isArray(payload.conflicts) ? payload.conflicts.length : undefined

      nextState = {
        ...base,
        knowledgeUpdatedAt: envelope.ts,
        knowledgeDecisionRequired: true,
        knowledgeDecision: payload,
        progressLog: trimMissionProgressLog([
          ...base.progressLog,
          {
            ts: envelope.ts,
            message: `Knowledge blocked · decision required${typeof conflicts === 'number' ? ` · ${conflicts} conflicts` : ''}${deltaId ? ` · ${deltaId.slice(0, 10)}…` : ''}`,
          },
        ]),
      }
      break
    }

    case 'MISSION_KNOWLEDGE_APPLIED': {
      const deltaId = typeof payload.delta_id === 'string'
        ? payload.delta_id.trim()
        : typeof payload.deltaId === 'string'
          ? payload.deltaId.trim()
          : ''

      nextState = {
        ...base,
        knowledgeUpdatedAt: envelope.ts,
        knowledgeDecisionRequired: false,
        knowledgeDecision: null,
        progressLog: trimMissionProgressLog([
          ...base.progressLog,
          {
            ts: envelope.ts,
            message: `Knowledge applied${deltaId ? ` · ${deltaId.slice(0, 10)}…` : ''}`,
          },
        ]),
      }
      break
    }

    case 'MISSION_KNOWLEDGE_ROLLED_BACK': {
      nextState = {
        ...base,
        knowledgeUpdatedAt: envelope.ts,
        knowledgeDecisionRequired: false,
        knowledgeDecision: null,
        progressLog: trimMissionProgressLog([
          ...base.progressLog,
          {
            ts: envelope.ts,
            message: 'Knowledge rolled back',
          },
        ]),
      }
      break
    }

    default: {
      console.warn('[mission-event] unknown event type:', envelope.type, envelope)
    }
  }

  currentMissionState = nextState
  notifyMissionListeners()
}

// ── Lifecycle: start / stop listening ───────────────────────────

let unlistenAgent: UnlistenFn | null = null
let unlistenMission: UnlistenFn | null = null

/**
 * Start listening to Rust backend events.
 * Call once at app startup (or when agent engine v2 is enabled).
 * Returns a cleanup function.
 */
export async function startBackendEventListeners(): Promise<() => void> {
  // Prevent double-subscribe
  await stopBackendEventListeners()

  unlistenAgent = await listen<AgentEventEnvelope>(AGENT_EVENT_CHANNEL, (event) => {
    try {
      dispatchAgentEvent(event.payload)
    } catch (err) {
      console.error('[agent-event] dispatch error:', err)
    }
  })

  unlistenMission = await listen<MissionEventEnvelope>(MISSION_EVENT_CHANNEL, (event) => {
    try {
      dispatchMissionEvent(event.payload)
    } catch (err) {
      console.error('[mission-event] dispatch error:', err)
    }
  })

  return stopBackendEventListeners
}

/**
 * Stop listening to Rust backend events.
 */
export async function stopBackendEventListeners(): Promise<void> {
  if (unlistenAgent) {
    unlistenAgent()
    unlistenAgent = null
  }
  if (unlistenMission) {
    unlistenMission()
    unlistenMission = null
  }

  currentMissionState = null
}

// ── Helpers ─────────────────────────────────────────────────────

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as Record<string, unknown>
}

function extractToolExposureMeta(payload: Record<string, unknown>) {
  const toolPackage = typeof payload.tool_package === 'string' ? payload.tool_package : undefined
  const routeReason = typeof payload.route_reason === 'string' ? payload.route_reason : undefined
  const fallbackFrom = typeof payload.fallback_from === 'string' ? payload.fallback_from : undefined
  const fallbackReason = typeof payload.fallback_reason === 'string' ? payload.fallback_reason : undefined
  const rolloutMode = typeof payload.rollout_mode === 'string' ? payload.rollout_mode : undefined
  const rolloutInCanary = typeof payload.rollout_in_canary === 'boolean'
    ? payload.rollout_in_canary
    : undefined
  const canaryPercent = typeof payload.canary_percent === 'number' ? payload.canary_percent : undefined
  const toolCallCount = typeof payload.tool_call_count === 'number' ? payload.tool_call_count : undefined
  const roundsExecuted = typeof payload.rounds_executed === 'number' ? payload.rounds_executed : undefined
  const fallbackOccurred = typeof payload.fallback_occurred === 'boolean'
    ? payload.fallback_occurred
    : undefined
  const exposedTools = Array.isArray(payload.exposed_tools)
    ? payload.exposed_tools.filter((value): value is string => typeof value === 'string')
    : []
  const skippedTools = Array.isArray(payload.skipped_tools)
    ? payload.skipped_tools
      .map((value) => asRecord(value))
      .filter((value): value is Record<string, unknown> => Boolean(value))
    : []

  if (
    !toolPackage
    && !routeReason
    && !fallbackFrom
    && !rolloutMode
    && toolCallCount === undefined
    && roundsExecuted === undefined
    && exposedTools.length === 0
    && skippedTools.length === 0
  ) {
    return undefined
  }

  return {
    tool_package: toolPackage,
    route_reason: routeReason,
    fallback_from: fallbackFrom,
    fallback_reason: fallbackReason,
    rollout_mode: rolloutMode,
    rollout_in_canary: rolloutInCanary,
    canary_percent: canaryPercent,
    tool_call_count: toolCallCount,
    rounds_executed: roundsExecuted,
    fallback_occurred: fallbackOccurred,
    exposed_tools: exposedTools,
    skipped_tools: skippedTools,
  }
}

function buildToolExposureSummary(payload: Record<string, unknown>) {
  const toolPackage = typeof payload.tool_package === 'string' ? payload.tool_package : 'unknown'
  const exposedTools = Array.isArray(payload.exposed_tools)
    ? payload.exposed_tools.filter((value): value is string => typeof value === 'string')
    : []
  const fallbackFrom = typeof payload.fallback_from === 'string' ? payload.fallback_from : undefined
  const base = `tool package · ${toolPackage} (${exposedTools.length})`
  return fallbackFrom ? `${base} · fallback ${fallbackFrom}` : base
}

function parseArgsPreview(raw: unknown): Record<string, unknown> {
  if (!raw) return {}
  const direct = asRecord(raw)
  if (direct) return direct
  if (typeof raw === 'string') {
    try {
      const parsed = JSON.parse(raw)
      return asRecord(parsed) || { _raw: raw }
    } catch {
      return { _raw: raw }
    }
  }
  return {}
}

function toToolTraceStage(raw: unknown): 'policy' | 'execute' | 'result' | undefined {
  switch (raw) {
    case 'policy':
    case 'execute':
    case 'result':
      return raw
    default:
      return undefined
  }
}

function normalizeStopReason(
  raw: unknown,
): 'success' | 'cancel' | 'error' | 'limit' | undefined {
  const s = String(raw ?? '')
  switch (s) {
    case 'success':
    case 'cancel':
    case 'error':
    case 'limit':
      return s
    case 'waiting_confirmation':
      return 'cancel'
    case 'waiting_askuser':
      return 'cancel'
    default:
      return 'success'
  }
}

