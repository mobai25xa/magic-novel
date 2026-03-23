import type { MissionUiState } from './types'

const MAX_MISSION_PROGRESS_ENTRIES = 40
const MAX_MISSION_WORKER_HISTORY = 8

export interface MissionBackedTurnBinding {
  missionId: string
  sessionId: string
  turnId: number
  projectPath: string
  userInput: string
  startedAt: number
}

export interface MissionBackedSessionState {
  sessionId: string
  activeMissionId?: string
  activeMissionState?: string
  latestMissionId?: string
  latestMissionState?: string
}

const missionStateByJobId: Record<string, MissionUiState> = {}
let currentMissionJobId: string | null = null
const bindingsByMissionId = new Map<string, MissionBackedTurnBinding>()
const missionStatusByMissionId = new Map<string, string>()
const activeMissionIdBySessionId = new Map<string, string>()
const latestMissionIdBySessionId = new Map<string, string>()
const sessionStateBySessionId = new Map<string, MissionBackedSessionState>()

type MissionStoreListener = {
  jobId?: string
  listener: (state: MissionUiState | null) => void
}

type MissionSessionStoreListener = {
  sessionId?: string
  listener: (state: MissionBackedSessionState) => void
}

const missionListeners: MissionStoreListener[] = []
const missionSessionListeners: MissionSessionStoreListener[] = []

function normalizeJobId(jobId: string) {
  return jobId.trim()
}

function normalizeSessionId(sessionId: string) {
  return sessionId.trim()
}

function normalizeMissionStatus(state: string) {
  return state.trim() || 'unknown'
}

function isBusyMissionStatus(state?: string) {
  return state === 'initializing' || state === 'running' || state === 'orchestrator_turn'
}

function getRelatedSessionIdsForMission(missionId: string) {
  const normalizedMissionId = normalizeJobId(missionId)
  if (!normalizedMissionId) {
    return []
  }

  const sessionIds = new Set<string>()
  const binding = bindingsByMissionId.get(normalizedMissionId)
  if (binding?.sessionId) {
    sessionIds.add(binding.sessionId)
  }

  for (const [sessionId, activeMissionId] of activeMissionIdBySessionId.entries()) {
    if (activeMissionId === normalizedMissionId) {
      sessionIds.add(sessionId)
    }
  }

  for (const [sessionId, latestMissionId] of latestMissionIdBySessionId.entries()) {
    if (latestMissionId === normalizedMissionId) {
      sessionIds.add(sessionId)
    }
  }

  return [...sessionIds]
}

function refreshMissionSessionState(sessionId: string) {
  const normalizedSessionId = normalizeSessionId(sessionId)
  if (!normalizedSessionId) {
    return
  }

  const activeMissionId = activeMissionIdBySessionId.get(normalizedSessionId)
  const latestMissionId = latestMissionIdBySessionId.get(normalizedSessionId)

  sessionStateBySessionId.set(normalizedSessionId, {
    sessionId: normalizedSessionId,
    activeMissionId,
    activeMissionState: activeMissionId ? missionStatusByMissionId.get(activeMissionId) : undefined,
    latestMissionId,
    latestMissionState: latestMissionId ? missionStatusByMissionId.get(latestMissionId) : undefined,
  })
}

function notifyMissionSessionListeners(changedSessionId?: string) {
  for (const entry of missionSessionListeners) {
    if (entry.sessionId) {
      entry.listener(getMissionBackedSessionState(entry.sessionId))
      continue
    }

    const sessionId = changedSessionId ? normalizeSessionId(changedSessionId) : ''
    entry.listener(sessionId ? getMissionBackedSessionState(sessionId) : { sessionId: '' })
  }
}

export function getMissionUiState(): MissionUiState | null {
  if (!currentMissionJobId) {
    return null
  }
  return missionStateByJobId[currentMissionJobId] ?? null
}

export function getMissionUiStateByJobId(jobId: string): MissionUiState | null {
  const normalizedJobId = normalizeJobId(jobId)
  if (!normalizedJobId) {
    return null
  }

  return missionStateByJobId[normalizedJobId] ?? null
}

export function getJobUiStateByJobId(jobId: string): MissionUiState | null {
  return getMissionUiStateByJobId(jobId)
}

export function subscribeMissionUiState(
  listener: (state: MissionUiState | null) => void,
  jobId?: string,
): () => void {
  const entry: MissionStoreListener = {
    listener,
    jobId: typeof jobId === 'string' && normalizeJobId(jobId) ? normalizeJobId(jobId) : undefined,
  }

  missionListeners.push(entry)
  return () => {
    const idx = missionListeners.indexOf(entry)
    if (idx >= 0) missionListeners.splice(idx, 1)
  }
}

export function subscribeMissionUiStateByJobId(
  jobId: string,
  listener: (state: MissionUiState | null) => void,
): () => void {
  return subscribeMissionUiState(listener, jobId)
}

export function subscribeJobUiStateByJobId(
  jobId: string,
  listener: (state: MissionUiState | null) => void,
): () => void {
  return subscribeMissionUiStateByJobId(jobId, listener)
}

export function subscribeMissionBackedSessionState(
  sessionId: string | undefined,
  listener: (state: MissionBackedSessionState) => void,
): () => void {
  const normalizedSessionId = normalizeSessionId(sessionId ?? '')
  const entry: MissionSessionStoreListener = {
    listener,
    sessionId: normalizedSessionId || undefined,
  }

  missionSessionListeners.push(entry)
  return () => {
    const idx = missionSessionListeners.indexOf(entry)
    if (idx >= 0) missionSessionListeners.splice(idx, 1)
  }
}

export function getMissionBackedSessionState(sessionId: string): MissionBackedSessionState {
  const normalizedSessionId = normalizeSessionId(sessionId)
  if (!normalizedSessionId) {
    return { sessionId: '' }
  }

  const cached = sessionStateBySessionId.get(normalizedSessionId)
  if (cached) {
    return cached
  }

  refreshMissionSessionState(normalizedSessionId)
  return sessionStateBySessionId.get(normalizedSessionId) ?? { sessionId: normalizedSessionId }
}

export function bindMissionBackedTurn(
  binding: MissionBackedTurnBinding,
  state = 'initializing',
) {
  const missionId = normalizeJobId(binding.missionId)
  const sessionId = normalizeSessionId(binding.sessionId)
  if (!missionId || !sessionId) {
    return
  }

  bindingsByMissionId.set(missionId, {
    ...binding,
    missionId,
    sessionId,
    projectPath: binding.projectPath.trim(),
    userInput: binding.userInput,
  })

  latestMissionIdBySessionId.set(sessionId, missionId)
  missionStatusByMissionId.set(missionId, normalizeMissionStatus(state))

  if (isBusyMissionStatus(state)) {
    activeMissionIdBySessionId.set(sessionId, missionId)
  } else if (activeMissionIdBySessionId.get(sessionId) === missionId) {
    activeMissionIdBySessionId.delete(sessionId)
  }

  refreshMissionSessionState(sessionId)
  notifyMissionSessionListeners(sessionId)
}

export function getMissionBackedTurnBinding(missionId: string) {
  const normalizedMissionId = normalizeJobId(missionId)
  if (!normalizedMissionId) {
    return undefined
  }

  return bindingsByMissionId.get(normalizedMissionId)
}

export function resolveMissionBackedTurnBinding(input: {
  jobId?: string
  missionId?: string
}) {
  const jobId = normalizeJobId(input.jobId ?? '')
  const missionId = normalizeJobId(input.missionId ?? '')

  return getMissionBackedTurnBinding(jobId)
    ?? (missionId && missionId !== jobId ? getMissionBackedTurnBinding(missionId) : undefined)
}

export function updateMissionBackedTurnState(missionId: string, state: string) {
  const normalizedMissionId = normalizeJobId(missionId)
  if (!normalizedMissionId) {
    return
  }

  const normalizedState = normalizeMissionStatus(state)
  missionStatusByMissionId.set(normalizedMissionId, normalizedState)

  const binding = bindingsByMissionId.get(normalizedMissionId)
  if (binding) {
    latestMissionIdBySessionId.set(binding.sessionId, normalizedMissionId)
    if (isBusyMissionStatus(normalizedState)) {
      activeMissionIdBySessionId.set(binding.sessionId, normalizedMissionId)
    } else if (activeMissionIdBySessionId.get(binding.sessionId) === normalizedMissionId) {
      activeMissionIdBySessionId.delete(binding.sessionId)
    }
  }

  const relatedSessionIds = getRelatedSessionIdsForMission(normalizedMissionId)
  for (const sessionId of relatedSessionIds) {
    refreshMissionSessionState(sessionId)
    notifyMissionSessionListeners(sessionId)
  }
}

export function clearMissionBackedTurnBinding(
  missionId: string,
  options?: { preserveLatest?: boolean },
) {
  const normalizedMissionId = normalizeJobId(missionId)
  if (!normalizedMissionId) {
    return
  }

  const preserveLatest = options?.preserveLatest ?? true
  const binding = bindingsByMissionId.get(normalizedMissionId)
  bindingsByMissionId.delete(normalizedMissionId)

  const relatedSessionIds = new Set<string>(getRelatedSessionIdsForMission(normalizedMissionId))
  if (binding?.sessionId) {
    relatedSessionIds.add(binding.sessionId)
  }

  for (const sessionId of relatedSessionIds) {
    if (activeMissionIdBySessionId.get(sessionId) === normalizedMissionId) {
      activeMissionIdBySessionId.delete(sessionId)
    }
    if (!preserveLatest && latestMissionIdBySessionId.get(sessionId) === normalizedMissionId) {
      latestMissionIdBySessionId.delete(sessionId)
    }
    refreshMissionSessionState(sessionId)
    notifyMissionSessionListeners(sessionId)
  }

  if (!preserveLatest) {
    missionStatusByMissionId.delete(normalizedMissionId)
  }
}

function notifyMissionListeners(changedJobId?: string) {
  for (const entry of missionListeners) {
    if (entry.jobId) {
      entry.listener(missionStateByJobId[entry.jobId] ?? null)
      continue
    }

    const snapshotJobId = changedJobId ?? currentMissionJobId ?? ''
    entry.listener(snapshotJobId ? missionStateByJobId[snapshotJobId] ?? null : null)
  }
}

export function resetMissionUiState() {
  currentMissionJobId = null
  for (const jobId of Object.keys(missionStateByJobId)) {
    delete missionStateByJobId[jobId]
  }
  bindingsByMissionId.clear()
  missionStatusByMissionId.clear()
  activeMissionIdBySessionId.clear()
  latestMissionIdBySessionId.clear()
  sessionStateBySessionId.clear()
  notifyMissionListeners()
  notifyMissionSessionListeners()
}

export function getOrCreateJobUiState(jobId: string): MissionUiState {
  const normalizedJobId = normalizeJobId(jobId)
  if (!normalizedJobId) {
    return createMissionUiState('')
  }

  return missionStateByJobId[normalizedJobId] ?? createMissionUiState(normalizedJobId)
}

export function getOrCreateMissionUiState(missionId: string): MissionUiState {
  return getOrCreateJobUiState(missionId)
}

export function commitJobUiState(nextState: MissionUiState) {
  const nextJobId = normalizeJobId(nextState.missionId)
  if (!nextJobId) {
    return
  }

  missionStateByJobId[nextJobId] = nextState
  currentMissionJobId = nextJobId
  notifyMissionListeners(nextJobId)
}

export function commitMissionUiState(nextState: MissionUiState) {
  commitJobUiState(nextState)
}

export function isTerminalMissionState(state: string) {
  return state === 'completed' || state === 'cancelled' || state === 'failed'
}

export function createMissionUiState(missionId: string): MissionUiState {
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
    fixupAttempt: undefined,
    fixupMessage: undefined,
    fixupUpdatedAt: undefined,
    fixupInProgress: undefined,
    knowledgeUpdatedAt: undefined,
    knowledgeDecisionRequired: undefined,
    knowledgeDecision: null,
    macroStateUpdatedAt: undefined,
    macroId: undefined,
    macroCurrentIndex: undefined,
    macroCurrentStage: undefined,
    macroChapterCount: undefined,
    macroCompletedCount: undefined,
    macroWorkflowKind: undefined,
    macroLastTransitionAt: undefined,
    macroChapterCompletedRef: undefined,
    macroChapterCompletedSummary: undefined,
    macroChapterCompletedAt: undefined,
  }
}

export function resetMissionTransientState(state: MissionUiState): MissionUiState {
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
    fixupAttempt: undefined,
    fixupMessage: undefined,
    fixupUpdatedAt: undefined,
    fixupInProgress: undefined,
    knowledgeUpdatedAt: undefined,
    knowledgeDecisionRequired: undefined,
    knowledgeDecision: null,
    macroStateUpdatedAt: undefined,
    macroChapterCompletedRef: undefined,
    macroChapterCompletedSummary: undefined,
    macroChapterCompletedAt: undefined,
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

export function upsertMissionWorkerStatus(
  workerStatuses: MissionUiState['workerStatuses'],
  workerId: string,
  nextStatus: MissionUiState['workerStatuses'][string],
): MissionUiState['workerStatuses'] {
  return pruneMissionWorkerStatuses({
    ...workerStatuses,
    [workerId]: nextStatus,
  })
}

export function trimMissionProgressLog(entries: MissionUiState['progressLog']): MissionUiState['progressLog'] {
  return entries.slice(-MAX_MISSION_PROGRESS_ENTRIES)
}
