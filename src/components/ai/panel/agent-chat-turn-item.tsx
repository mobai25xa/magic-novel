import { memo, useMemo } from 'react'

import { cancelCurrentChatTurn } from '@/lib/agent-chat/runtime'
import type { AgentUiTimelineEvent, AgentUiToolStep } from '@/lib/agent-chat/types'
import { useAgentChatStore } from '@/state/agent'
import type { AiChatViewMode } from '@/state/settings'

import { TurnCard } from '../turn-card'

type AgentChatTurnItemProps = {
  turnId: number
  viewMode: AiChatViewMode
  sessionId: string
  running: boolean
  sessionRuntimeState: 'ready' | 'running' | 'suspended_confirmation' | 'suspended_askuser' | 'completed' | 'failed' | 'cancelled' | 'degraded'
  sessionCanResume: boolean
  sessionReadonlyReason?: string
  onRetryStep: (turnId: number, callId: string) => void
}

const EMPTY_TOOL_STEPS: AgentUiToolStep[] = []
const EMPTY_EVENTS: AgentUiTimelineEvent[] = []

function AgentChatTurnItemImpl(input: AgentChatTurnItemProps) {
  const turnState = useAgentChatStore((state) => state.turnById[input.turnId])
  const answerText = useAgentChatStore((state) => state.answerByTurnId[input.turnId] || '')
  const thinkingText = useAgentChatStore((state) => state.thinkingByTurnId[input.turnId] || '')
  const toolSteps = useAgentChatStore((state) => state.stepsByTurnId[input.turnId] || EMPTY_TOOL_STEPS)
  const events = useAgentChatStore((state) => state.eventsByTurnId[input.turnId] || EMPTY_EVENTS)
  const userMessages = useAgentChatStore((state) => state.messages)
  const timelineSnapshot = useAgentChatStore((state) => state.committedTimelineByTurnId[input.turnId])
  const pendingAskUser = useAgentChatStore((state) => state.pendingAskUser)
  const resolveAskUserRequest = useAgentChatStore((state) => state.resolveAskUserRequest)
  const cancelAskUserRequest = useAgentChatStore((state) => state.cancelAskUserRequest)
  const handleCancelAskUser = (callId: string) => {
    if (pendingAskUser?.callId === callId) {
      cancelCurrentChatTurn()
      return
    }

    cancelAskUserRequest(callId)
  }

  const userText = useMemo(() => {
    const userMessage = userMessages
      .filter((message) => message.role === 'user' && message.turn === input.turnId)
      .at(-1)
    return userMessage?.content || ''
  }, [input.turnId, userMessages])

  const sortedToolSteps = useMemo(() => [...toolSteps].sort((a, b) => {
    if (a.startedAt !== b.startedAt) {
      return a.startedAt - b.startedAt
    }
    return a.callId.localeCompare(b.callId)
  }), [toolSteps])

  const turnView = useMemo(() => {
    if (!turnState) {
      return null
    }

    return {
      state: turnState,
      userText,
      answerText,
      thinkingText,
      toolSteps: sortedToolSteps,
      events,
    }
  }, [answerText, events, sortedToolSteps, thinkingText, turnState, userText])

  if (!turnView) {
    return null
  }

  return (
    <TurnCard
      view={turnView}
      viewMode={input.viewMode}
      sessionId={input.sessionId}
      running={input.running}
      sessionRuntimeState={input.sessionRuntimeState}
      sessionCanResume={input.sessionCanResume}
      sessionReadonlyReason={input.sessionReadonlyReason}
      onRetryStep={input.onRetryStep}
      pendingAskUser={pendingAskUser}
      onResolveAskUser={resolveAskUserRequest}
      onCancelAskUser={handleCancelAskUser}
      timelineSnapshot={timelineSnapshot}
    />
  )
}

export const AgentChatTurnItem = memo(AgentChatTurnItemImpl)
