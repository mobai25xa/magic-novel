import { useEffect, useMemo, useRef, useSyncExternalStore, type Dispatch, type SetStateAction } from 'react'

import { useAgentChatStore } from '@/state/agent'
import { useProjectStore } from '@/state/project'

import type { AgentPanelError } from '../../agent-chat-panel-utils'
import { createSessionPanelActions } from '../session/agent-chat-panel-session-actions'
import {
  ensureSessionRuntimeCapability,
  getSessionRuntimeCapabilitySnapshot,
  resetSessionRuntimeCapabilitySnapshot,
  syncSessionRuntimeCapability,
  subscribeSessionRuntimeCapability,
  type SessionRuntimeCapability,
} from '../session/agent-chat-panel-session-actions-store'

type SessionActionsInput = {
  setLastError: Dispatch<SetStateAction<AgentPanelError | null>>
}

export function usePanelSessionState(input: SessionActionsInput) {
  const sessionList = useAgentChatStore((state) => state.sessionList)
  const currentSessionMeta = useAgentChatStore((state) => state.currentSessionMeta)
  const isSessionLoading = useAgentChatStore((state) => state.isSessionLoading)
  const sessionError = useAgentChatStore((state) => state.sessionError)
  const wasSessionResumed = useAgentChatStore((state) => state.wasSessionResumed)
  const activeSessionId = useAgentChatStore((state) => state.session_id)
  const liveRuntimeState = useAgentChatStore((state) => state.sessionRuntimeState)
  const liveHydrationStatus = useAgentChatStore((state) => state.sessionHydrationStatus)
  const liveCanContinue = useAgentChatStore((state) => state.sessionCanContinue)
  const liveCanResume = useAgentChatStore((state) => state.sessionCanResume)
  const liveReadonlyReason = useAgentChatStore((state) => state.sessionReadonlyReason)
  const liveWarnings = useAgentChatStore((state) => state.sessionWarnings)

  const runtimeCapabilitySnapshot = useSyncExternalStore(
    subscribeSessionRuntimeCapability,
    getSessionRuntimeCapabilitySnapshot,
    () => getSessionRuntimeCapabilitySnapshot(),
  )

  const currentSessionId = currentSessionMeta?.session_id ?? activeSessionId
  const hasLiveRuntimeCapability = Boolean(currentSessionMeta)
    || liveRuntimeState !== undefined
    || liveHydrationStatus !== undefined
    || liveCanContinue
    || liveCanResume
    || Boolean(liveReadonlyReason)
    || liveWarnings.length > 0

  const liveSessionRuntimeCapability = useMemo<SessionRuntimeCapability>(() => ({
    runtimeState: liveRuntimeState ?? 'ready',
    canContinue: liveCanContinue,
    canResume: liveCanResume,
    readonlyReason: liveReadonlyReason,
    hydrationStatus: liveHydrationStatus,
    warnings: liveWarnings,
  }), [
    liveCanContinue,
    liveCanResume,
    liveHydrationStatus,
    liveReadonlyReason,
    liveRuntimeState,
    liveWarnings,
  ])

  useEffect(() => {
    if (!currentSessionId) {
      return
    }

    if (currentSessionMeta) {
      return
    }

    const hasSnapshot = Boolean(runtimeCapabilitySnapshot.bySessionId[currentSessionId])
    if (hasSnapshot) {
      return
    }

    ensureSessionRuntimeCapability({
      sessionId: currentSessionId,
    })
  }, [currentSessionId, currentSessionMeta, runtimeCapabilitySnapshot.bySessionId])

  useEffect(() => {
    if (!currentSessionId || !hasLiveRuntimeCapability) {
      return
    }

    syncSessionRuntimeCapability({
      sessionId: currentSessionId,
      capability: liveSessionRuntimeCapability,
    })
  }, [
    currentSessionId,
    hasLiveRuntimeCapability,
    liveSessionRuntimeCapability,
  ])

  const sessionActions = useMemo(() => createSessionPanelActions({
    setLastError: input.setLastError,
  }), [input.setLastError])

  const sessionRuntimeCapability: SessionRuntimeCapability = hasLiveRuntimeCapability
    ? liveSessionRuntimeCapability
    : ((currentSessionId ? runtimeCapabilitySnapshot.bySessionId[currentSessionId] : undefined)
      ?? runtimeCapabilitySnapshot.current)

  const sessionRuntimeCapabilityBySessionId = useMemo(() => {
    if (!currentSessionId || !hasLiveRuntimeCapability) {
      return runtimeCapabilitySnapshot.bySessionId
    }

    return {
      ...runtimeCapabilitySnapshot.bySessionId,
      [currentSessionId]: liveSessionRuntimeCapability,
    }
  }, [
    currentSessionId,
    hasLiveRuntimeCapability,
    liveSessionRuntimeCapability,
    runtimeCapabilitySnapshot.bySessionId,
  ])

  return {
    sessionList,
    currentSessionMeta,
    isSessionLoading,
    sessionError,
    wasSessionResumed,
    sessionActions,
    sessionRuntimeCapability,
    sessionRuntimeCapabilityBySessionId,
  }
}

export function useLoadSessionListEffect(input: {
  enabled: boolean
  loadSessionList: () => Promise<void>
}) {
  const { enabled, loadSessionList } = input
  const projectPath = useProjectStore((state) => state.projectPath)
  const resetForProjectSwitch = useAgentChatStore((state) => state.resetForProjectSwitch)
  const prevProjectPathRef = useRef<string | null>(null)

  useEffect(() => {
    if (prevProjectPathRef.current !== projectPath) {
      resetForProjectSwitch()
      resetSessionRuntimeCapabilitySnapshot()
      prevProjectPathRef.current = projectPath
    }
  }, [projectPath, resetForProjectSwitch])

  useEffect(() => {
    if (!enabled || !projectPath) {
      return
    }

    void loadSessionList()
  }, [enabled, projectPath, loadSessionList])
}
