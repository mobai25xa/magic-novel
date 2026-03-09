import { logConcurrency } from '@/agent/telemetry'
import { formatUnknownError } from '@/lib/error-utils'

import { useProjectStore } from '@/stores/project-store'
import { useSettingsStore } from '@/stores/settings-store'

import {
  buildUserUiMessage,
  createClientRequestId,
  normalizeUserInput,
} from './runtime-helpers'
import { useAgentChatStore } from './store'
import {
  agentTurnStartClient,
  agentTurnCancelClient,
  agentTurnResumeClient,
  type AgentTurnStartOutput,
  type ApprovalMode,
  type CapabilityMode,
  type ClarificationMode,
} from '@/platform/tauri/clients/agent-engine-client'
import type { RunChatTurnOptions } from './types'
import {
  startBackendEventListeners,
  stopBackendEventListeners,
} from './runtime-backend-events'

const MAX_CONCURRENT_TURNS = 1
const DEFAULT_APPROVAL_MODE: ApprovalMode = 'confirm_writes'
const DEFAULT_CAPABILITY_MODE: CapabilityMode = 'writing'
const DEFAULT_CLARIFICATION_MODE: ClarificationMode = 'interactive'

let activeTurnCount = 0
let backendListenersStarted = false

function buildSessionGateError(code: 'E_AGENT_SESSION_READONLY' | 'E_AGENT_SESSION_SUSPENDED', message: string) {
  const error = new Error(message) as Error & { code?: string }
  error.name = code
  error.code = code
  return error
}

function assertSessionCanContinueForTurn() {
  const store = useAgentChatStore.getState()

  if (
    store.sessionRuntimeState === 'suspended_confirmation'
    || store.sessionRuntimeState === 'suspended_askuser'
    || store.sessionCanResume
  ) {
    throw buildSessionGateError(
      'E_AGENT_SESSION_SUSPENDED',
      'Session is suspended and must be resumed before sending a new message.',
    )
  }

  if (store.sessionRuntimeState === 'degraded' || !store.sessionCanContinue) {
    throw buildSessionGateError(
      'E_AGENT_SESSION_READONLY',
      store.sessionReadonlyReason || 'Session is read-only and cannot continue.',
    )
  }
}

async function ensurePersistedSessionForTurn() {
  const projectPath = useProjectStore.getState().projectPath
  if (!projectPath) {
    return
  }

  const store = useAgentChatStore.getState()
  await store.ensurePersistedSession({
    projectPath,
    activeChapterPath: store.active_chapter_path,
  })
}

/**
 * Ensure backend event listeners are running when v2 engine is active.
 * Idempotent — safe to call multiple times.
 */
async function ensureBackendListeners() {
  if (backendListenersStarted) return
  backendListenersStarted = true
  await startBackendEventListeners()
}

export async function ensureBackendListenersStarted() {
  await ensureBackendListeners()
}

export function normalizeRunModes(options: RunChatTurnOptions = {}) {
  const settings = useSettingsStore.getState()

  let capabilityMode = options.capabilityMode ?? settings.capabilityMode ?? DEFAULT_CAPABILITY_MODE
  let approvalMode = options.approvalMode ?? settings.approvalMode ?? DEFAULT_APPROVAL_MODE
  const clarificationMode = options.clarificationMode ?? DEFAULT_CLARIFICATION_MODE

  if (approvalMode !== 'confirm_writes' && approvalMode !== 'auto') {
    approvalMode = DEFAULT_APPROVAL_MODE
  }
  if (capabilityMode !== 'writing' && capabilityMode !== 'planning') {
    capabilityMode = DEFAULT_CAPABILITY_MODE
  }

  return {
    approvalMode,
    capabilityMode,
    clarificationMode,
  }
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

function bindAuthoritativeTurn(input: {
  clientRequestId: string
  turnId: number
}) {
  const store = useAgentChatStore.getState()
  const result = store.bindPendingTurnRequest({
    clientRequestId: input.clientRequestId,
    turn: input.turnId,
  })

  if (!result.cancelRequested) {
    store.setSessionRuntimeCapability({
      runtimeState: 'running',
      canContinue: false,
      canResume: false,
      readonlyReason: undefined,
    })
  }

  if (result.cancelRequested) {
    store.setStateStatus('idle')
    agentTurnCancelClient({
      session_id: store.session_id,
      turn_id: input.turnId,
    }).catch((error) => {
      console.error('[agent-engine-v2] cancel-after-bind failed:', error)
    })
  }

  return result
}

/**
 * Execute a turn via Rust agent_engine v2.
 * The Rust side spawns the agent loop and emits streaming events.
 * The event subscription layer (`runtime-backend-events.ts`) maps those
 * events to store actions, so we only need to fire-and-forget here.
 */
async function executeRustEngineTurn(
  input: string,
  sessionId: string,
  clientRequestId: string,
  options?: RunChatTurnOptions,
): Promise<AgentTurnStartOutput> {
  await ensureBackendListeners()

  const settings = useSettingsStore.getState()
  const projectPath = useProjectStore.getState().projectPath || ''
  const normalizedModes = normalizeRunModes(options)

  return agentTurnStartClient({
    session_id: sessionId,
    client_request_id: clientRequestId,
    user_text: input,
    project_path: projectPath,
    model: settings.openaiModel || undefined,
    provider: 'openai-compatible',
    base_url: settings.openaiBaseUrl || undefined,
    api_key: settings.openaiApiKey || undefined,
    active_chapter_path: useAgentChatStore.getState().active_chapter_path || undefined,
    approval_mode: normalizedModes.approvalMode,
    capability_mode: normalizedModes.capabilityMode,
    clarification_mode: normalizedModes.clarificationMode,
  })
}


export function cancelCurrentChatTurn() {
  const store = useAgentChatStore.getState()

  const pendingClientRequestId = getOldestPendingClientRequestId()
  if (pendingClientRequestId && !store.boundTurnByClientRequestId[pendingClientRequestId]) {
    store.requestPendingTurnCancellation(pendingClientRequestId)
    if (store.pendingAskUser) {
      store.cancelAskUserRequest(store.pendingAskUser.callId)
    }
    return
  }

  if (store.pendingAskUser) {
    const { callId, turn } = store.pendingAskUser
    store.cancelAskUserRequest(callId)
    agentTurnCancelClient({
      session_id: store.session_id,
      turn_id: turn,
    }).catch((err) => {
      console.error('[agent-engine-v2] cancel askuser failed:', err)
    })
    return
  }

  const turnId = store.turn
  if (turnId) {
    agentTurnCancelClient({
      session_id: store.session_id,
      turn_id: turnId,
    }).catch((err) => {
      console.error('[agent-engine-v2] cancel failed:', err)
    })
  }
}

export async function runChatTurn(inputText: string, options: RunChatTurnOptions = {}): Promise<string> {
  const input = normalizeUserInput(inputText)
  if (!input) return ''

  await ensurePersistedSessionForTurn()
  assertSessionCanContinueForTurn()

  const store = useAgentChatStore.getState()
  const sessionId = store.session_id

  if (activeTurnCount >= MAX_CONCURRENT_TURNS) {
    logConcurrency({ sessionId, queued: 1, running: activeTurnCount })
    throw new Error('E_AGENT_CONCURRENCY_LIMIT')
  }

  const clientRequestId = createClientRequestId()
  const userMsg = buildUserUiMessage(input)

  store.startPendingTurnRequest({
    clientRequestId,
    userMessage: userMsg,
  })
  store.setStateStatus('thinking')
  store.pushLlmMessage({ role: 'user', content: input })

  activeTurnCount += 1
  logConcurrency({ sessionId, queued: 0, running: activeTurnCount })

  try {
    const startAck = await executeRustEngineTurn(input, sessionId, clientRequestId, options)
    bindAuthoritativeTurn({
      clientRequestId: startAck.client_request_id ?? clientRequestId,
      turnId: startAck.turn_id,
    })
    return ''
  } catch (error) {
    const currentStore = useAgentChatStore.getState()
    const errorText = formatUnknownError(error)

    currentStore.failPendingTurnRequest({
      clientRequestId,
      removePendingMessages: true,
    })

    const turnId = currentStore.boundTurnByClientRequestId[clientRequestId]
    if (typeof turnId === 'number') {
      currentStore.setTurnPhase(turnId, 'failed', {
        error: errorText,
      })
      currentStore.pushTurnEvent(turnId, {
        type: 'TURN_FAILED',
        ts: Date.now(),
        summary: errorText,
      })
      currentStore.commitTurnTimelineSnapshot(turnId)
      currentStore.setSessionRuntimeCapability({
        runtimeState: 'failed',
        canContinue: true,
        canResume: false,
        readonlyReason: undefined,
      })
    }

    currentStore.setStateStatus('idle')

    throw error
  } finally {
    activeTurnCount = Math.max(0, activeTurnCount - 1)
    logConcurrency({ sessionId, queued: 0, running: activeTurnCount })
  }
}

/**
 * Resume a turn that is suspended waiting for confirmation.
 * Called from the UI when user approves/denies a tool call.
 */
export { agentTurnResumeClient as resumeAgentTurn }

/**
 * Tear down backend event listeners (e.g. on settings change or unmount).
 */
export async function teardownBackendEventListeners() {
  backendListenersStarted = false
  await stopBackendEventListeners()
}
