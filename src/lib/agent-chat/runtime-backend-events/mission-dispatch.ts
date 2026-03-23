import { createMessageId } from '@/agent/utils'
import { missionGetStatusFeature } from '@/features/agent-chat'
import {
  getMissionResultEntriesPreferJobSnapshot,
  isTaskResultSuccessful,
  type MissionGetStatusOutput,
  type MissionResultEntry,
} from '@/lib/tauri-commands/mission'
import type { JobEvent } from '@/types/agent-job'

import { useAgentChatStore } from '../store'
import { appendPersistedSessionEventsClient } from '../session/session-client'
import {
  toSessionMessageEvent,
  toSessionTurnFinalEvent,
} from '../session/session-event-builders'
import type { MissionEventEnvelope, MissionUiState } from './types'

import {
  clearMissionBackedTurnBinding,
  commitMissionUiState,
  getOrCreateMissionUiState,
  isTerminalMissionState,
  resetMissionTransientState,
  resolveMissionBackedTurnBinding,
  trimMissionProgressLog,
  updateMissionBackedTurnState,
  upsertMissionWorkerStatus,
} from './mission-store'

type MissionEventReducer = (base: MissionUiState, envelope: MissionEventEnvelope) => MissionUiState

const finalizingMissionIds = new Set<string>()

function normalizeIdentifier(value: unknown): string {
  return typeof value === 'string' ? value.trim() : ''
}

function resolveMissionEnvelopeJobId(envelope: MissionEventEnvelope): string {
  const payloadJobId = normalizeIdentifier(envelope.payload.job_id)
  return payloadJobId || normalizeIdentifier(envelope.mission_id)
}

function getMissionBindingForEnvelope(envelope: MissionEventEnvelope) {
  const missionId = normalizeIdentifier(envelope.mission_id)
  const jobId = resolveMissionEnvelopeJobId(envelope)
  return resolveMissionBackedTurnBinding({ jobId, missionId })
}

export function toJobEvent(envelope: MissionEventEnvelope): JobEvent {
  const payload = envelope.payload
  const featureTaskId = typeof payload.feature_id === 'string' ? payload.feature_id.trim() : ''
  const taskId = typeof payload.task_id === 'string'
    ? payload.task_id.trim()
    : featureTaskId

  return {
    schema_version: envelope.schema_version,
    event_type: envelope.type,
    job_id: resolveMissionEnvelopeJobId(envelope),
    task_id: taskId,
    payload,
    ts: envelope.ts,
  }
}

function dedupeStrings(values: string[]) {
  const result: string[] = []
  const seen = new Set<string>()

  for (const value of values) {
    const normalized = value.trim()
    if (!normalized || seen.has(normalized)) {
      continue
    }
    seen.add(normalized)
    result.push(normalized)
  }

  return result
}

function summarizeResultEntries(results: MissionResultEntry[]) {
  const summaries = results
    .map((entry) => entry.summary.trim())
    .filter(Boolean)

  const artifacts = dedupeStrings(results.flatMap((entry) => entry.artifacts || [])).slice(0, 4)
  const issues = dedupeStrings(results.flatMap((entry) => entry.issues || [])).slice(0, 4)
  const nextActions = dedupeStrings(results.flatMap((entry) => entry.next_actions || [])).slice(0, 4)
  const sections: string[] = []

  if (summaries.length > 0) {
    sections.push(summaries.join('\n\n'))
  }

  if (artifacts.length > 0) {
    sections.push(`Artifacts:\n- ${artifacts.join('\n- ')}`)
  }

  if (issues.length > 0) {
    sections.push(`Remaining issues:\n- ${issues.join('\n- ')}`)
  }

  if (nextActions.length > 0) {
    sections.push(`Next actions:\n- ${nextActions.join('\n- ')}`)
  }

  return sections
}

function summarizeMissionResult(status: MissionGetStatusOutput) {
  const resultEntries = getMissionResultEntriesPreferJobSnapshot(status)
  const successfulResults = resultEntries.filter((entry) => isTaskResultSuccessful(entry))
  const unsuccessfulResults = resultEntries.filter((entry) => !isTaskResultSuccessful(entry))
  const sections = summarizeResultEntries(successfulResults)

  if (sections.length === 0) {
    const completedFeatures = status.features.features
      .filter((feature) => feature.status === 'completed')
      .map((feature) => feature.description.trim())
      .filter(Boolean)
      .slice(0, 3)

    if (completedFeatures.length > 0) {
      sections.push(`Completed:\n- ${completedFeatures.join('\n- ')}`)
    }
  }

  if (sections.length === 0 && unsuccessfulResults.length > 0) {
    sections.push(...summarizeResultEntries(unsuccessfulResults))
  }

  if (sections.length === 0) {
    return 'Mission completed. Open MissionPanel to inspect the produced artifacts and task results.'
  }

  return sections.join('\n\n')
}

function isVisibleMissionSession(sessionId: string) {
  return useAgentChatStore.getState().session_id === sessionId
}

function isCurrentMissionTurn(sessionId: string, turnId: number) {
  const store = useAgentChatStore.getState()
  return store.session_id === sessionId && store.turn === turnId
}

function setStoreIdleWithoutCompletingTurn(turnId: number) {
  const store = useAgentChatStore.getState()
  if (store.turn !== turnId) {
    return
  }

  store.setTurn(0)
  store.setStateStatus('idle')
  store.setTurn(turnId)
}

function handleMissionWorkerTimelineEvent(envelope: MissionEventEnvelope) {
  const binding = getMissionBindingForEnvelope(envelope)
  if (!binding || !isVisibleMissionSession(binding.sessionId)) {
    return
  }

  const payload = envelope.payload
  const store = useAgentChatStore.getState()
  const workerId = typeof payload.worker_id === 'string' ? payload.worker_id.trim() : ''
  const featureId = typeof payload.feature_id === 'string' ? payload.feature_id.trim() : ''
  const featureText = featureId ? ` (${featureId})` : ''

  if (envelope.type === 'WORKER_STARTED') {
    store.pushTurnEvent(binding.turnId, {
      type: 'WORKER_STARTED',
      ts: envelope.ts,
      summary: workerId ? `Worker ${workerId}${featureText} started.` : `Worker${featureText} started.`,
    })
    return
  }

  if (envelope.type === 'WORKER_COMPLETED') {
    const summary = typeof payload.summary === 'string' ? payload.summary.trim() : ''
    store.pushTurnEvent(binding.turnId, {
      type: 'WORKER_COMPLETED',
      ts: envelope.ts,
      summary: summary || (workerId ? `Worker ${workerId}${featureText} completed.` : `Worker${featureText} completed.`),
    })
  }
}

function syncMissionRuntimeState(envelope: MissionEventEnvelope) {
  if (envelope.type !== 'MISSION_STATE_CHANGED') {
    return
  }

  const binding = getMissionBindingForEnvelope(envelope)
  if (!binding) {
    return
  }

  const newState = typeof envelope.payload.new_state === 'string'
    ? envelope.payload.new_state
    : 'unknown'
  const missionId = normalizeIdentifier(envelope.mission_id)
  const jobId = resolveMissionEnvelopeJobId(envelope)

  if (missionId) {
    updateMissionBackedTurnState(missionId, newState)
  }
  if (jobId && jobId !== missionId) {
    updateMissionBackedTurnState(jobId, newState)
  }

  if (!isVisibleMissionSession(binding.sessionId)) {
    return
  }

  const store = useAgentChatStore.getState()

  if ((newState === 'initializing' || newState === 'running' || newState === 'orchestrator_turn')
    && isCurrentMissionTurn(binding.sessionId, binding.turnId)) {
    store.setStateStatus('thinking')
    store.setSessionRuntimeCapability({
      runtimeState: 'running',
      canContinue: false,
      canResume: false,
      readonlyReason: undefined,
    })
    return
  }

  if (newState === 'paused' || newState === 'awaiting_input') {
    if (isCurrentMissionTurn(binding.sessionId, binding.turnId)) {
      setStoreIdleWithoutCompletingTurn(binding.turnId)
      store.setSessionRuntimeCapability({
        runtimeState: 'ready',
        canContinue: true,
        canResume: false,
        readonlyReason: undefined,
      })
    }
  }
}

async function finalizeMissionBackedTurn(input: {
  jobId: string
  missionId: string
  terminalState: string
}) {
  const missionId = normalizeIdentifier(input.missionId)
  const jobId = normalizeIdentifier(input.jobId)
  const terminalState = input.terminalState
  const binding = resolveMissionBackedTurnBinding({ jobId, missionId })
  const finalizeKey = binding?.missionId ?? missionId ?? jobId

  if (!binding || !finalizeKey || finalizingMissionIds.has(finalizeKey)) {
    return
  }

  finalizingMissionIds.add(finalizeKey)

  try {
    const finishedAt = Date.now()
    const latencyMs = Math.max(0, finishedAt - binding.startedAt)
    const stopReason = terminalState === 'completed' ? 'success' : 'error'
    let assistantText = ''

    if (terminalState === 'completed') {
      try {
        const status = await missionGetStatusFeature(binding.projectPath, binding.missionId)
        assistantText = summarizeMissionResult(status)
      } catch (error) {
        console.warn('[mission-event] failed to load mission result summary:', error)
        assistantText = 'Mission completed, but the final result summary could not be loaded. Open MissionPanel to inspect artifacts and worker output.'
      }
    }

    const sessionEvents = [] as Parameters<typeof appendPersistedSessionEventsClient>[0]['events']
    const assistantMessage = assistantText
      ? {
          id: createMessageId(),
          role: 'assistant' as const,
          content: assistantText,
          ts: finishedAt,
          turn: binding.turnId,
        }
      : null

    if (assistantMessage) {
      sessionEvents.push(toSessionMessageEvent({
        sessionId: binding.sessionId,
        message: assistantMessage,
      }))
    }

    if (isVisibleMissionSession(binding.sessionId)) {
      const store = useAgentChatStore.getState()
      const isCurrentTurn = isCurrentMissionTurn(binding.sessionId, binding.turnId)

      if (assistantMessage) {
        store.setTurnPhase(binding.turnId, 'synthesizing')
        store.pushTurnEvent(binding.turnId, {
          type: 'SYNTHESIS_STARTED',
          ts: Math.max(binding.startedAt, finishedAt - 1),
          summary: 'Collecting mission result.',
        })
        store.addUiMessage(assistantMessage)
        store.pushLlmMessage({ role: 'assistant', content: assistantText })
      }

      if (terminalState === 'completed') {
        if (isCurrentTurn) {
          store.setLastStopReason('success')
          store.setLastTurnLatency(latencyMs)
        }
        store.setTurnPhase(binding.turnId, 'completed', {
          stopReason: 'success',
          finishedAt,
        })
        store.pushTurnEvent(binding.turnId, {
          type: 'TURN_COMPLETED',
          ts: finishedAt,
          summary: 'Mission completed.',
        })
      } else {
        const errorText = `Mission ended in state ${terminalState}.`
        if (isCurrentTurn) {
          store.setLastStopReason('error')
          store.setLastTurnLatency(latencyMs)
        }
        store.setTurnPhase(binding.turnId, 'failed', {
          stopReason: 'error',
          error: errorText,
          finishedAt,
        })
        store.pushTurnEvent(binding.turnId, {
          type: 'TURN_FAILED',
          ts: finishedAt,
          summary: errorText,
        })
      }

      store.commitTurnTimelineSnapshot(binding.turnId)

      if (isCurrentTurn) {
        store.setStateStatus('idle')
        store.setSessionRuntimeCapability({
          runtimeState: terminalState === 'completed' ? 'ready' : 'failed',
          canContinue: true,
          canResume: false,
          readonlyReason: undefined,
        })
      }
    }

    sessionEvents.push(toSessionTurnFinalEvent({
      sessionId: binding.sessionId,
      turnId: binding.turnId,
      stopReason,
      latencyMs,
      ts: finishedAt,
    }))

    await appendPersistedSessionEventsClient({
      projectPath: binding.projectPath,
      sessionId: binding.sessionId,
      events: sessionEvents,
    })
  } finally {
    clearMissionBackedTurnBinding(binding.missionId)
    finalizingMissionIds.delete(finalizeKey)
  }
}

const MISSION_EVENT_HANDLERS: Record<string, MissionEventReducer> = {
  MISSION_STATE_CHANGED: (base, envelope) => {
    const payload = envelope.payload
    const newState = String(payload.new_state ?? 'unknown')
    const next = { ...base, state: newState }
    return isTerminalMissionState(newState) ? resetMissionTransientState(next) : next
  },

  MISSION_FEATURES_CHANGED: (base, envelope) => {
    const payload = envelope.payload
    const featureId = String(payload.feature_id ?? '').trim()
    return { ...base, currentFeatureId: featureId || undefined }
  },

  MISSION_PROGRESS_ENTRY: (base, envelope) => {
    const payload = envelope.payload
    return {
      ...base,
      progressLog: trimMissionProgressLog([
        ...base.progressLog,
        { ts: envelope.ts, message: String(payload.message ?? '') },
      ]),
    }
  },

  WORKER_STARTED: (base, envelope) => {
    const payload = envelope.payload
    const workerId = String(payload.worker_id ?? '')
    const featureId = String(payload.feature_id ?? '')
    if (!workerId.trim()) {
      return base
    }
    return {
      ...base,
      workerStatuses: upsertMissionWorkerStatus(base.workerStatuses, workerId, {
        featureId,
        status: 'running',
        updatedAt: envelope.ts,
      }),
    }
  },

  WORKER_COMPLETED: (base, envelope) => {
    const payload = envelope.payload
    const workerId = String(payload.worker_id ?? '')
    const featureId = String(payload.feature_id ?? '')
    if (!workerId.trim()) {
      return base
    }
    return {
      ...base,
      workerStatuses: upsertMissionWorkerStatus(base.workerStatuses, workerId, {
        featureId,
        status: payload.ok ? 'completed' : 'failed',
        summary: String(payload.summary ?? ''),
        updatedAt: envelope.ts,
      }),
    }
  },

  // No-op for now, could update last-seen timestamp.
  MISSION_HEARTBEAT: (base) => base,

  MISSION_LAYER1_UPDATED: (base, envelope) => ({ ...base, layer1UpdatedAt: envelope.ts }),

  MISSION_CONTEXTPACK_BUILT: (base, envelope) => ({ ...base, contextPackBuiltAt: envelope.ts }),

  MISSION_REVIEW_RECORDED: (base, envelope) => ({
    ...base,
    reviewUpdatedAt: envelope.ts,
    reviewDecisionRequired: false,
    reviewDecision: null,
    fixupInProgress: false,
  }),

  MISSION_REVIEW_DECISION_REQUIRED: (base, envelope) => ({
    ...base,
    reviewUpdatedAt: envelope.ts,
    reviewDecisionRequired: true,
    reviewDecision: envelope.payload,
  }),

  MISSION_FIXUP_PROGRESS: (base, envelope) => {
    const payload = envelope.payload
    const attempt = typeof payload.attempt === 'number'
      ? payload.attempt
      : Number(String(payload.attempt ?? ''))

    return {
      ...base,
      fixupAttempt: Number.isFinite(attempt) ? attempt : base.fixupAttempt,
      fixupMessage: typeof payload.message === 'string' ? payload.message : base.fixupMessage,
      fixupUpdatedAt: envelope.ts,
      fixupInProgress: true,
    }
  },

  MISSION_KNOWLEDGE_PROPOSED: (base, envelope) => ({
    ...base,
    knowledgeUpdatedAt: envelope.ts,
    knowledgeDecisionRequired: false,
    knowledgeDecision: null,
  }),

  MISSION_KNOWLEDGE_DECISION_REQUIRED: (base, envelope) => ({
    ...base,
    knowledgeUpdatedAt: envelope.ts,
    knowledgeDecisionRequired: true,
    knowledgeDecision: envelope.payload,
  }),

  MISSION_KNOWLEDGE_APPLIED: (base, envelope) => ({
    ...base,
    knowledgeUpdatedAt: envelope.ts,
    knowledgeDecisionRequired: false,
    knowledgeDecision: null,
  }),

  MISSION_KNOWLEDGE_ROLLED_BACK: (base, envelope) => ({
    ...base,
    knowledgeUpdatedAt: envelope.ts,
  }),

  MISSION_MACRO_STATE_UPDATED: (base, envelope) => {
    const payload = envelope.payload
    const macroId = typeof payload.macro_id === 'string' ? payload.macro_id.trim() : undefined
    const currentIndex = typeof payload.current_index === 'number' ? payload.current_index : undefined
    const currentStage = typeof payload.current_stage === 'string' ? payload.current_stage.trim() : undefined
    const chapterCount = typeof payload.chapter_count === 'number' ? payload.chapter_count : undefined
    const completedCount = typeof payload.completed_count === 'number' ? payload.completed_count : undefined
    const workflowKind = typeof payload.workflow_kind === 'string' ? payload.workflow_kind.trim() : undefined
    const lastTransitionAt = typeof payload.last_transition_at === 'number' ? payload.last_transition_at : undefined

    return {
      ...base,
      macroStateUpdatedAt: envelope.ts,
      macroId: macroId ?? base.macroId,
      macroCurrentIndex: currentIndex ?? base.macroCurrentIndex,
      macroCurrentStage: currentStage ?? base.macroCurrentStage,
      macroChapterCount: chapterCount ?? base.macroChapterCount,
      macroCompletedCount: completedCount ?? base.macroCompletedCount,
      macroWorkflowKind: workflowKind ?? base.macroWorkflowKind,
      macroLastTransitionAt: lastTransitionAt ?? base.macroLastTransitionAt,
    }
  },

  MISSION_MACRO_CHAPTER_COMPLETED: (base, envelope) => {
    const payload = envelope.payload
    return {
      ...base,
      macroStateUpdatedAt: envelope.ts,
      macroChapterCompletedRef: typeof payload.chapter_ref === 'string' ? payload.chapter_ref : undefined,
      macroChapterCompletedSummary: typeof payload.summary === 'string' ? payload.summary : undefined,
      macroChapterCompletedAt: envelope.ts,
    }
  },
}

export function dispatchMissionEvent(envelope: MissionEventEnvelope) {
  const jobEvent = toJobEvent(envelope)
  const base = getOrCreateMissionUiState(jobEvent.job_id)
  const handler = MISSION_EVENT_HANDLERS[jobEvent.event_type]

  if (!handler) {
    console.warn('[mission-event] unknown event type:', envelope.type, envelope)
    commitMissionUiState(base)
    return
  }

  commitMissionUiState(handler(base, envelope))
  syncMissionRuntimeState(envelope)
  handleMissionWorkerTimelineEvent(envelope)

  if (envelope.type === 'MISSION_STATE_CHANGED') {
    const newState = typeof envelope.payload.new_state === 'string'
      ? envelope.payload.new_state
      : 'unknown'

    if (newState === 'completed' || newState === 'failed' || newState === 'cancelled') {
      void finalizeMissionBackedTurn({
        jobId: jobEvent.job_id,
        missionId: envelope.mission_id,
        terminalState: newState,
      }).catch((error) => {
        console.error('[mission-event] finalize mission-backed turn failed:', error)
      })
    }
  }
}
