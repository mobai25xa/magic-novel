import { useEffect, useState, useSyncExternalStore, type Dispatch, type SetStateAction } from 'react'
import type { AgentStateStatus } from '@/agent/types'

import { useAgentChatStore } from '@/state/agent'
import { useProjectStore } from '@/state/project'
import { useSettingsStore, type AiChatViewMode, type ApprovalMode, type CapabilityMode } from '@/state/settings'

import type { AgentSessionMeta } from '@/lib/agent-chat/session'

import { useStreamingTimer } from '@/hooks/use-streaming-timer'
import { useTranslation } from '@/hooks/use-translation'
import {
  getMissionBackedSessionState,
  subscribeMissionBackedSessionState,
} from '@/lib/agent-chat/runtime-backend-events/mission-store'
import { ensureBackendListenersStarted } from '@/lib/agent-chat/runtime'
import type { ChatContext } from '../input/chat-context-types'
import type { AgentPanelError } from '../agent-chat-panel-utils'
import { useLatestTurnSignature } from './agent-chat-panel-signature'
import { useAgentChatSessionId } from './session/agent-chat-panel-session'
import type { SessionRuntimeCapability } from './session/agent-chat-panel-session-actions-store'
import { usePanelContexts } from './state/agent-chat-panel-state-contexts'
import { usePanelModelState } from './state/agent-chat-panel-state-model'
import {
  useLoadSessionListEffect,
  usePanelSessionState,
} from './state/agent-chat-panel-state-session'

export type AgentChatPanelRuntimeState = SessionRuntimeCapability['runtimeState']
export type AgentChatPanelHydrationStatus = SessionRuntimeCapability['hydrationStatus']

type SessionReadonlyReason =
  | 'runtime_state_unavailable'
  | 'historical_suspended_session_without_runtime_snapshot'
  | 'provider_credentials_unavailable_for_resume'

function normalizeReadonlyReason(reason?: string): SessionReadonlyReason {
  if (reason === 'historical_suspended_session_without_runtime_snapshot') {
    return reason
  }

  if (reason === 'provider_credentials_unavailable_for_resume') {
    return reason
  }

  return 'runtime_state_unavailable'
}

function mapReadonlyReasonToI18nKey(reason?: string):
  | 'sessionReadOnlyReasonMissingRuntime'
  | 'sessionReadOnlyReasonLegacySession'
  | 'sessionReadOnlyReasonConfigMissing'
  | undefined {
  if (!reason) {
    return undefined
  }

  const normalized = normalizeReadonlyReason(reason)
  if (normalized === 'historical_suspended_session_without_runtime_snapshot') {
    return 'sessionReadOnlyReasonLegacySession'
  }

  if (normalized === 'provider_credentials_unavailable_for_resume') {
    return 'sessionReadOnlyReasonConfigMissing'
  }

  return 'sessionReadOnlyReasonMissingRuntime'
}

function resolveInputPlaceholder(input: {
  labels: AgentPanelLabels
  running: boolean
  runtimeState: AgentChatPanelRuntimeState
  canContinue: boolean
  readonlyReason?: string
}) {
  if (input.running) {
    return `${input.labels.panel.generating}...`
  }

  if (input.canContinue) {
    return input.labels.panel.inputPlaceholder
  }

  if (input.runtimeState === 'suspended_confirmation' || input.runtimeState === 'suspended_askuser') {
    return input.labels.panel.sessionInputDisabledSuspended
  }

  const reasonKey = mapReadonlyReasonToI18nKey(input.readonlyReason)
  const reasonText = reasonKey
    ? input.labels.panel[reasonKey]
    : input.labels.panel.sessionReadOnlyReasonMissingRuntime

  return `${input.labels.panel.sessionInputDisabledReadOnly}：${reasonText}`
}

function resolveRuntimeCapabilityWarnings(input: {
  labels: AgentPanelLabels
  warnings: string[]
  readonlyReason?: string
}) {
  if (input.warnings.length > 0) {
    return input.warnings
  }

  const reasonKey = mapReadonlyReasonToI18nKey(input.readonlyReason)
  if (!reasonKey) {
    return []
  }

  return [input.labels.panel[reasonKey]]
}

function normalizeRuntimeStateForHistory(input: {
  state: AgentChatPanelRuntimeState
  canContinue: boolean
  canResume: boolean
}): AgentChatPanelRuntimeState {
  if (input.canContinue) {
    return 'ready'
  }

  if (input.canResume) {
    if (input.state === 'suspended_askuser') {
      return 'suspended_askuser'
    }
    return 'suspended_confirmation'
  }

  return 'degraded'
}

function mapHistoryStateLabel(input: {
  labels: AgentPanelLabels
  state: AgentChatPanelRuntimeState
  canContinue: boolean
  canResume: boolean
}) {
  const normalized = normalizeRuntimeStateForHistory(input)
  if (normalized === 'ready') {
    return input.labels.panel.historyStateInteractive
  }

  if (normalized === 'suspended_confirmation') {
    return input.labels.panel.historyStateSuspendedConfirmation
  }

  if (normalized === 'suspended_askuser') {
    return input.labels.panel.historyStateSuspendedAskUser
  }

  return input.labels.panel.historyStateReadOnly
}

function mapHydrationStatus(status: SessionRuntimeCapability['hydrationStatus']): AgentChatPanelHydrationStatus {
  return status
}

type AgentPanelLabels = import('@/i18n/locales/zh').Translations['ai']
type AgentChatContextLabels = import('@/i18n/locales/zh').Translations['aiChat']

type AgentChatPanelStateOutputInput = {
  input: string
  setInput: Dispatch<SetStateAction<string>>
  running: boolean
  approvalMode: ApprovalMode
  capabilityMode: CapabilityMode
  lastError: AgentPanelError | null
  setLastError: Dispatch<SetStateAction<AgentPanelError | null>>
  sessionId: string
  turnIds: number[]
  latestTurnSignature: string
  availableModels: string[]
  selectedModel: string
  handleSelectModel: (model: string) => void
  aiChatViewMode: AiChatViewMode
  setAiChatViewMode: (mode: AiChatViewMode) => void
  sessionPersistenceEnabled: boolean
  sessionList: AgentSessionMeta[]
  currentSessionMeta?: AgentSessionMeta
  isSessionLoading: boolean
  sessionError: string | null
  wasSessionResumed: boolean
  sessionRuntimeState: AgentChatPanelRuntimeState
  sessionHydrationStatus?: AgentChatPanelHydrationStatus
  sessionCanContinue: boolean
  sessionCanResume: boolean
  sessionReadonlyReason?: string
  sessionWarnings: string[]
  sessionInputDisabled: boolean
  sessionInputPlaceholder: string
  historyStateBySessionId: Record<string, string>
  canStartNewSession: boolean
  startNewSession: () => Promise<void>
  openHistoryPage: () => void
  closeHistoryPage: () => void
  historyPageOpen: boolean
  resumeSession: (sessionId: string) => Promise<void>
  renameSession: (sessionId: string, title: string) => Promise<void>
  deleteSession: (sessionId: string) => Promise<void>
  contexts: ChatContext[]
  addContext: (context: ChatContext) => void
  removeContext: (contextId: string) => void
  clearContexts: () => void
  streamingElapsedSeconds: number
  streamingElapsedTime: string
  showStreamingTimer: boolean
  projectPath: string
  missionId: string
  missionBusy: boolean
  missionLocked: boolean
  missionDispatching: boolean
  setMissionDispatching: Dispatch<SetStateAction<boolean>>
  activeChapterPath?: string
  activeSkill?: string
  labels: AgentPanelLabels
  chatLabels: AgentChatContextLabels
}

function createPanelStateOutput(input: AgentChatPanelStateOutputInput) {
  return {
    ...input,
  }
}

function isStreamingStatus(status: AgentStateStatus) {
  return status === 'thinking' || status === 'executing_tool' || status === 'compacting'
}

export function useAgentChatPanelState() {
  const [input, setInput] = useState('')
  const [lastError, setLastError] = useState<AgentPanelError | null>(null)
  const [historyPageOpen, setHistoryPageOpen] = useState(false)
  const [missionDispatching, setMissionDispatching] = useState(false)

  const approvalMode = useSettingsStore((state) => state.approvalMode)
  const capabilityMode = useSettingsStore((state) => state.capabilityMode)

  const sessionId = useAgentChatSessionId()
  const turnIds = useAgentChatStore((state) => state.turnOrder)
  const stateStatus = useAgentChatStore((state) => state.stateStatus)
  const activeChapterPath = useAgentChatStore((state) => state.active_chapter_path)
  const activeSkill = useAgentChatStore((state) => state.activeSkill)
  const latestTurnSignature = useLatestTurnSignature()
  const projectPath = useProjectStore((state) => state.projectPath ?? '')
  const { translations } = useTranslation()

  const chatRunning = stateStatus !== 'idle'
  const streamingActive = isStreamingStatus(stateStatus)

  const modelState = usePanelModelState()
  const sessionState = usePanelSessionState({ setLastError })
  const contextState = usePanelContexts()
  const {
    elapsedSeconds: streamingElapsedSeconds,
    isRunning: streamingTimerRunning,
    start: startStreamingTimer,
    stop: stopStreamingTimer,
    formattedTime: streamingElapsedTime,
  } = useStreamingTimer()

  useEffect(() => {
    if (streamingActive) {
      if (!streamingTimerRunning) {
        startStreamingTimer()
      }
      return
    }

    if (streamingTimerRunning) {
      stopStreamingTimer()
    }
  }, [streamingActive, streamingTimerRunning, startStreamingTimer, stopStreamingTimer])

  useLoadSessionListEffect({
    enabled: modelState.sessionPersistenceEnabled,
    loadSessionList: sessionState.sessionActions.loadSessionList,
  })

  useEffect(() => {
    void ensureBackendListenersStarted()
  }, [])

  const missionSessionState = useSyncExternalStore(
    (listener) => subscribeMissionBackedSessionState(sessionId, listener),
    () => getMissionBackedSessionState(sessionId),
    () => getMissionBackedSessionState(sessionId),
  )
  const missionStateValue = missionSessionState.activeMissionState ?? 'unknown'
  const liveMissionBusy = Boolean(
    missionSessionState.activeMissionId
      && missionStateValue !== 'paused'
      && missionStateValue !== 'awaiting_input'
      && missionStateValue !== 'unknown'
      && missionStateValue !== 'completed'
      && missionStateValue !== 'failed'
      && missionStateValue !== 'cancelled',
  )
  const missionLocked = missionDispatching || liveMissionBusy
  const missionBusy = missionDispatching || liveMissionBusy
  const running = chatRunning
  const missionId = projectPath
    ? (missionSessionState.latestMissionId ?? missionSessionState.activeMissionId ?? '')
    : ''

  const runtimeCapability = sessionState.sessionRuntimeCapability
  const sessionRuntimeState = runtimeCapability.runtimeState
  const sessionHydrationStatus = mapHydrationStatus(runtimeCapability.hydrationStatus)
  const sessionCanContinue = runtimeCapability.canContinue
  const sessionCanResume = runtimeCapability.canResume
  const sessionReadonlyReason = runtimeCapability.readonlyReason
  const sessionWarnings = resolveRuntimeCapabilityWarnings({
    labels: translations.ai,
    warnings: runtimeCapability.warnings,
    readonlyReason: runtimeCapability.readonlyReason,
  })

  const sessionInputDisabled =
    chatRunning
    || sessionRuntimeState === 'running'
    || !sessionCanContinue
  const sessionInputPlaceholder = resolveInputPlaceholder({
    labels: translations.ai,
    running: chatRunning || sessionRuntimeState === 'running',
    runtimeState: sessionRuntimeState,
    canContinue: sessionCanContinue,
    readonlyReason: sessionReadonlyReason,
  })
  const canStartNewSession = !(
    chatRunning
    || sessionRuntimeState === 'running'
    || sessionState.isSessionLoading
  )

  const historyStateBySessionId = Object.fromEntries(
    sessionState.sessionList.map((item) => {
      const capability = sessionState.sessionRuntimeCapabilityBySessionId[item.session_id]
      if (!capability) {
        if (item.session_id === sessionState.currentSessionMeta?.session_id) {
          const currentLabel = mapHistoryStateLabel({
            labels: translations.ai,
            state: sessionRuntimeState,
            canContinue: sessionCanContinue,
            canResume: sessionCanResume,
          })
          return [item.session_id, currentLabel] as const
        }

        return [item.session_id, translations.ai.panel.historyStateReadOnly] as const
      }

      const label = mapHistoryStateLabel({
        labels: translations.ai,
        state: capability.runtimeState,
        canContinue: capability.canContinue,
        canResume: capability.canResume,
      })

      return [item.session_id, label] as const
    }),
  ) as Record<string, string>

  return createPanelStateOutput({
    input,
    setInput,
    running,
    approvalMode,
    capabilityMode,
    lastError,
    setLastError,
    sessionId,
    turnIds,
    latestTurnSignature,
    availableModels: modelState.availableModels,
    selectedModel: modelState.selectedModel,
    handleSelectModel: modelState.handleSelectModel,
    aiChatViewMode: modelState.aiChatViewMode,
    setAiChatViewMode: modelState.setAiChatViewMode,
    sessionPersistenceEnabled: modelState.sessionPersistenceEnabled,
    sessionList: sessionState.sessionList,
    currentSessionMeta: sessionState.currentSessionMeta,
    isSessionLoading: sessionState.isSessionLoading,
    sessionError: sessionState.sessionError,
    wasSessionResumed: sessionState.wasSessionResumed,
    sessionRuntimeState,
    sessionHydrationStatus,
    sessionCanContinue,
    sessionCanResume,
    sessionReadonlyReason,
    sessionWarnings,
    sessionInputDisabled,
    sessionInputPlaceholder,
    historyStateBySessionId,
    canStartNewSession,
    startNewSession: sessionState.sessionActions.startNewSession,
    openHistoryPage: () => setHistoryPageOpen(true),
    closeHistoryPage: () => setHistoryPageOpen(false),
    historyPageOpen,
    resumeSession: sessionState.sessionActions.resumeSession,
    renameSession: sessionState.sessionActions.renameSession,
    deleteSession: sessionState.sessionActions.deleteSession,
    contexts: contextState.contexts,
    addContext: contextState.addContext,
    removeContext: contextState.removeContext,
    clearContexts: contextState.clearContexts,
    streamingElapsedSeconds,
    streamingElapsedTime,
    showStreamingTimer: streamingActive,
    projectPath,
    missionId,
    missionBusy,
    missionLocked,
    missionDispatching,
    setMissionDispatching,
    activeChapterPath,
    activeSkill,
    labels: translations.ai,
    chatLabels: translations.aiChat,
  })
}
