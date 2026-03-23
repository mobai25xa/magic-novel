import { logConcurrency } from '@/agent/telemetry'
import { buildStandardAiProviderConfig, buildStandardAiProviderInput } from '@/features/standard-ai-consumer'
import { formatUnknownError } from '@/lib/error-utils'
import { missionCreate, missionStart, type Feature } from '@/lib/tauri-commands/mission'

import { useProjectStore } from '@/stores/project-store'
import { useSettingsStore } from '@/stores/settings-store'

import {
  buildUserUiMessage,
  createClientRequestId,
  normalizeUserInput,
} from './runtime-helpers'
import { appendPersistedSessionEventsClient } from './session/session-client'
import {
  toSessionMessageEvent,
  toSessionTurnFinalEvent,
} from './session/session-event-builders'
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
import {
  bindMissionBackedTurn,
  clearMissionBackedTurnBinding,
  commitMissionUiState,
  createMissionUiState,
  updateMissionBackedTurnState,
} from './runtime-backend-events/mission-store'

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

function buildMissionDispatchError(code: string, message: string) {
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

function normalizeMissionWritePath(rawPath?: string) {
  if (typeof rawPath !== 'string') {
    return undefined
  }

  const normalized = rawPath
    .trim()
    .replace(/\\/g, '/')
    .replace(/^chapter:/, '')
    .replace(/^manuscripts\//, '')
    .replace(/^\/+/, '')

  if (!normalized || normalized.split('/').some((segment) => segment === '..')) {
    return undefined
  }

  return normalized
}

function createChatMissionTitle(userText: string) {
  const normalized = userText.replace(/\s+/g, ' ').trim()
  if (!normalized) {
    return 'Editor Chat Mission'
  }

  return normalized.length <= 64
    ? `Editor Chat · ${normalized}`
    : `Editor Chat · ${normalized.slice(0, 61)}...`
}

function resolveNextMissionTurnId() {
  const store = useAgentChatStore.getState()
  const turnOrderMax = store.turnOrder.length > 0
    ? Math.max(...store.turnOrder)
    : 0

  return Math.max(
    1,
    typeof store.turn === 'number' && store.turn > 0 ? store.turn + 1 : 0,
    typeof store.sessionNextTurnId === 'number' && store.sessionNextTurnId > 0 ? store.sessionNextTurnId : 0,
    typeof store.sessionLastTurn === 'number' && store.sessionLastTurn >= 0 ? store.sessionLastTurn + 1 : 0,
    turnOrderMax + 1,
  )
}

async function appendMissionSessionEvents(input: {
  projectPath: string
  sessionId: string
  events: Parameters<typeof appendPersistedSessionEventsClient>[0]['events']
}) {
  if (!input.projectPath.trim()) {
    return
  }

  await appendPersistedSessionEventsClient({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    events: input.events,
  })
}

async function prepareMissionBackedTurn(input: {
  projectPath: string
  userText: string
}) {
  const store = useAgentChatStore.getState()
  const turnId = resolveNextMissionTurnId()
  const userMessage = buildUserUiMessage(input.userText, turnId)
  const startedAt = userMessage.ts

  store.markTurnStarted(turnId)
  store.addUiMessage(userMessage)
  store.pushLlmMessage({ role: 'user', content: input.userText })
  store.setStateStatus('thinking')
  store.pushTurnEvent(turnId, {
    type: 'TURN_STARTED',
    ts: startedAt,
    summary: 'Mission dispatched from editor chat.',
  })
  store.pushTurnEvent(turnId, {
    type: 'PLAN_STARTED',
    ts: startedAt + 1,
    summary: 'Preparing mission execution.',
  })
  store.setSessionRuntimeCapability({
    runtimeState: 'running',
    canContinue: false,
    canResume: false,
    readonlyReason: undefined,
  })

  await appendMissionSessionEvents({
    projectPath: input.projectPath,
    sessionId: store.session_id,
    events: [
      toSessionMessageEvent({
        sessionId: store.session_id,
        message: userMessage,
      }),
    ],
  })

  return {
    sessionId: store.session_id,
    turnId,
    startedAt,
  }
}

async function failMissionBackedTurnStart(input: {
  projectPath: string
  sessionId: string
  turnId: number
  startedAt: number
  errorText: string
}) {
  const finishedAt = Date.now()
  const store = useAgentChatStore.getState()
  const isVisibleSession = store.session_id === input.sessionId

  if (isVisibleSession) {
    store.setLastStopReason('error')
    store.setLastTurnLatency(Math.max(0, finishedAt - input.startedAt))
    store.setTurnPhase(input.turnId, 'failed', {
      stopReason: 'error',
      error: input.errorText,
      finishedAt,
    })
    store.pushTurnEvent(input.turnId, {
      type: 'TURN_FAILED',
      ts: finishedAt,
      summary: input.errorText,
    })
    store.commitTurnTimelineSnapshot(input.turnId)
    store.setStateStatus('idle')
    store.setSessionRuntimeCapability({
      runtimeState: 'failed',
      canContinue: true,
      canResume: false,
      readonlyReason: undefined,
    })
  }

  await appendMissionSessionEvents({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    events: [
      toSessionTurnFinalEvent({
        sessionId: input.sessionId,
        turnId: input.turnId,
        stopReason: 'error',
        latencyMs: Math.max(0, finishedAt - input.startedAt),
        ts: finishedAt,
      }),
    ],
  })
}

function buildChatMissionFeature(input: {
  userText: string
  sessionId: string
  capabilityMode: CapabilityMode
  activeChapterPath?: string
  activeSkill?: string
}): Feature {
  const writePath = input.capabilityMode === 'writing'
    ? normalizeMissionWritePath(input.activeChapterPath)
    : undefined
  const featureId = `chat_${Date.now()}`

  const preconditions = [
    `Origin session: ${input.sessionId}`,
    input.activeSkill ? `Active skill: ${input.activeSkill}` : '',
    writePath ? `Primary chapter scope: ${writePath}` : '',
  ].filter(Boolean)

  const expectedBehavior = input.capabilityMode === 'planning'
    ? [
        'Focus on analysis, planning, and scoped recommendations before any concrete edits.',
        writePath
          ? `Treat ${writePath} as the primary inspection scope when grounding the plan.`
          : 'Inspect only the project files needed to ground the plan.',
        'Return a concise execution summary and explicitly call out unresolved blockers.',
      ]
    : [
        writePath
          ? `Keep file writes scoped to ${writePath} unless the request clearly requires a broader change.`
          : 'Inspect relevant project context before making changes, and keep edits tightly scoped.',
        'Use the assigned worker profile and approved tools to complete the request directly when safe.',
        'Return a concise summary of changes, touched artifacts, and unresolved issues.',
      ]

  const verificationSteps = [
    writePath
      ? `Verify the result against ${writePath} and any directly related artifacts.`
      : 'Verify the result against the files and artifacts touched during execution.',
    'Report what changed, what was not changed, and any follow-up work that remains.',
  ]

  return {
    id: featureId,
    status: 'pending',
    description: input.userText,
    skill: input.activeSkill?.trim() || '',
    preconditions,
    depends_on: [],
    expected_behavior: expectedBehavior,
    verification_steps: verificationSteps,
    write_paths: writePath ? [writePath] : [],
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

  const projectPath = useProjectStore.getState().projectPath || ''
  const normalizedModes = normalizeRunModes(options)
  const providerInput = buildStandardAiProviderInput({ clientRequestId })

  return agentTurnStartClient({
    session_id: sessionId,
    client_request_id: clientRequestId,
    user_text: input,
    project_path: projectPath,
    ...providerInput,
    active_chapter_path: useAgentChatStore.getState().active_chapter_path || undefined,
    approval_mode: normalizedModes.approvalMode,
    capability_mode: normalizedModes.capabilityMode,
    clarification_mode: normalizedModes.clarificationMode,
  })
}

export async function startChatMission(
  inputText: string,
  options: RunChatTurnOptions = {},
): Promise<string> {
  const input = normalizeUserInput(inputText)
  if (!input) return ''

  await ensurePersistedSessionForTurn()
  assertSessionCanContinueForTurn()
  await ensureBackendListeners()

  const projectPath = (useProjectStore.getState().projectPath || '').trim()
  if (!projectPath) {
    throw buildMissionDispatchError(
      'E_MISSION_PROJECT_REQUIRED',
      'Open a project before starting a mission from editor chat.',
    )
  }

  const modes = normalizeRunModes(options)
  const providerConfig = buildStandardAiProviderConfig()
  const turnContext = await prepareMissionBackedTurn({
    projectPath,
    userText: input,
  })
  const store = useAgentChatStore.getState()

  const feature = buildChatMissionFeature({
    userText: input,
    sessionId: turnContext.sessionId,
    capabilityMode: modes.capabilityMode,
    activeChapterPath: store.active_chapter_path,
    activeSkill: store.activeSkill,
  })
  let createdMissionId = ''

  try {
    const created = await missionCreate({
      project_path: projectPath,
      title: createChatMissionTitle(input),
      mission_text: input,
      features: [feature],
    })
    createdMissionId = created.mission_id

    bindMissionBackedTurn({
      missionId: createdMissionId,
      sessionId: turnContext.sessionId,
      turnId: turnContext.turnId,
      projectPath,
      userInput: input,
      startedAt: turnContext.startedAt,
    })

    const startedAt = Date.now()
    commitMissionUiState({
      ...createMissionUiState(createdMissionId),
      state: 'initializing',
      currentFeatureId: feature.id,
      progressLog: [{
        ts: startedAt,
        message: `Mission created from editor chat (${turnContext.sessionId})`,
      }],
    })

    await missionStart({
      project_path: projectPath,
      mission_id: createdMissionId,
      max_workers: 1,
      provider: providerConfig.provider,
      model: providerConfig.model,
      base_url: providerConfig.base_url,
      api_key: providerConfig.api_key,
      parent_session_id: turnContext.sessionId,
      parent_turn_id: turnContext.turnId,
      delegate_transport: 'in_process',
    })

    return createdMissionId
  } catch (error) {
    const errorText = formatUnknownError(error)
    if (createdMissionId) {
      updateMissionBackedTurnState(createdMissionId, 'failed')
      commitMissionUiState({
        ...createMissionUiState(createdMissionId),
        state: 'failed',
        currentFeatureId: feature.id,
        progressLog: [{
          ts: Date.now(),
          message: `Mission start failed: ${errorText}`,
        }],
      })
      clearMissionBackedTurnBinding(createdMissionId)
    }
    await failMissionBackedTurnStart({
      projectPath,
      sessionId: turnContext.sessionId,
      turnId: turnContext.turnId,
      startedAt: turnContext.startedAt,
      errorText,
    })
    throw error
  }
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
