import { useCallback, useState } from 'react'

import type { AgentPendingAskUserRequest } from '@/agent/types'
import type { AgentUiTurnView } from '@/lib/agent-chat/types'
import type { AiChatViewMode } from '@/state/settings'
import { useAgentChatStore } from '@/state/agent'
import { resumeAgentTurnFeature } from '@/features/agent-chat'

import { TurnCardContent } from './message/turn-card-content'
import { TurnCardUserBlock } from './message/turn-card-user-block'
import { TimelineBlocksRenderer } from './timeline/TimelineBlocksRenderer'
import { resolveTurnTimeline } from './timeline/resolve-turn-timeline'
import { useTurnRenderMetric } from './turn-metrics'
import { TurnErrorCard } from './error/TurnErrorCard'
import { classifyTurnError } from './error/classify-error'
import { useAiTranslations } from './ai-hooks'
import { TurnExecutionToggle } from './turn-execution-toggle'
import { TurnExecutionTimeline } from './turn-execution-timeline'

type TurnCardProps = {
  view: AgentUiTurnView
  viewMode: AiChatViewMode
  sessionId: string
  running: boolean
  sessionRuntimeState: 'ready' | 'running' | 'suspended_confirmation' | 'suspended_askuser' | 'completed' | 'failed' | 'cancelled' | 'degraded'
  sessionCanResume: boolean
  sessionReadonlyReason?: string
  onRetryStep: (turnId: number, callId: string) => void
  onRetryTurn?: () => void
  onOpenSettings?: () => void
  pendingAskUser?: AgentPendingAskUserRequest
  onResolveAskUser: (callId: string, answers: import('@/agent/types').AgentAskUserAnswer[]) => void
  onCancelAskUser: (callId: string) => void
  timelineSnapshot?: unknown
}

function isTurnActive(phase: AgentUiTurnView['state']['phase']) {
  return phase === 'queued'
    || phase === 'planning'
    || phase === 'tool_running'
    || phase === 'synthesizing'
    || phase === 'compacting'
}

function readTurnToolExposure(view: AgentUiTurnView) {
  const eventMeta = [...view.events]
    .reverse()
    .map((event) => event.meta)
    .find((meta) => meta && typeof meta.tool_package === 'string')

  const errorDetail = view.state.turnError?.detail

  const toolPackage = typeof eventMeta?.tool_package === 'string'
    ? eventMeta.tool_package
    : errorDetail?.tool_package
  const routeReason = typeof eventMeta?.route_reason === 'string'
    ? eventMeta.route_reason
    : errorDetail?.route_reason
  const fallbackFrom = typeof eventMeta?.fallback_from === 'string'
    ? eventMeta.fallback_from
    : errorDetail?.fallback_from
  const fallbackReason = typeof eventMeta?.fallback_reason === 'string'
    ? eventMeta.fallback_reason
    : errorDetail?.fallback_reason
  const exposedTools = Array.isArray(eventMeta?.exposed_tools)
    ? eventMeta.exposed_tools.filter((value): value is string => typeof value === 'string')
    : errorDetail?.exposed_tools ?? []

  if (!toolPackage && !routeReason && !fallbackFrom && exposedTools.length === 0) {
    return null
  }

  return {
    toolPackage,
    routeReason,
    fallbackFrom,
    fallbackReason,
    exposedTools,
  }
}

function stripThinkingPrefix(answer: string, thinking: string) {
  const normalizedAnswer = answer.trimStart()
  const normalizedThinking = thinking.trim()
  if (!normalizedThinking) {
    return answer
  }

  if (normalizedAnswer.startsWith(normalizedThinking)) {
    const prefixIndex = answer.indexOf(normalizedThinking)
    if (prefixIndex >= 0) {
      return answer.slice(prefixIndex + normalizedThinking.length).trimStart()
    }
  }

  return answer
}

export function TurnCard(input: TurnCardProps) {
  const ai = useAiTranslations()

  useTurnRenderMetric({
    sessionId: input.sessionId,
    turnId: input.view.state.turn,
    stepCount: input.view.toolSteps.length,
  })

  const activeTurn = isTurnActive(input.view.state.phase)
  const turnId = input.view.state.turn
  const allowResumeAction = input.sessionCanResume
    && (input.sessionRuntimeState === 'suspended_confirmation' || input.sessionRuntimeState === 'suspended_askuser')

  const timeline = resolveTurnTimeline({
    turn: turnId,
    events: input.view.events,
    toolSteps: input.view.toolSteps,
    answerText: input.view.answerText,
    thinkingText: input.view.thinkingText,
    running: activeTurn && input.running,
    phase: input.view.state.phase,
    snapshot: input.timelineSnapshot,
  })

  const turnPendingAskUser = input.pendingAskUser && input.pendingAskUser.turn === turnId
    ? input.pendingAskUser
    : undefined
  const toolExposure = input.viewMode === 'debug' ? readTurnToolExposure(input.view) : null

  const executionBlocks = timeline.blocks.filter((block) => block.type !== 'assistant_segment')
  const toolCallCount = executionBlocks.filter((block) => block.type === 'tool_call').length
  const hasThinkingPanel = executionBlocks.some((block) => block.type === 'thinking_panel' && block.hasContent)
  const hasWaitingConfirmation = input.view.toolSteps.some(
    (step) => step.status === 'waiting_confirmation' && step.progress === 'waiting_confirmation',
  )
  const forceOpenExecution = input.viewMode === 'compact'
    && allowResumeAction
    && (hasWaitingConfirmation || Boolean(turnPendingAskUser))

  const [executionOpenByUser, setExecutionOpenByUser] = useState(() => forceOpenExecution)
  const executionOpen = forceOpenExecution ? true : executionOpenByUser

  const handleToggleExecution = useCallback(() => {
    if (forceOpenExecution) {
      return
    }
    setExecutionOpenByUser((value) => !value)
  }, [forceOpenExecution])

  const handleRetry = useCallback(() => {
    // Retry is handled at the panel level; this is a placeholder for future wiring
  }, [])

  const handleApprove = useCallback((_callId: string) => {
    if (!allowResumeAction) {
      return
    }

    const store = useAgentChatStore.getState()
    resumeAgentTurnFeature({
      session_id: store.session_id,
      turn_id: turnId,
      resume_input: { kind: 'confirmation', allowed: true },
    }).catch((err) => {
      console.error('[agent-engine-v2] resume (approve) failed:', err)
    })
  }, [allowResumeAction, turnId])

  const handleSkip = useCallback((_callId: string) => {
    if (!allowResumeAction) {
      return
    }

    const store = useAgentChatStore.getState()
    resumeAgentTurnFeature({
      session_id: store.session_id,
      turn_id: turnId,
      resume_input: { kind: 'confirmation', allowed: false },
    }).catch((err) => {
      console.error('[agent-engine-v2] resume (skip) failed:', err)
    })
  }, [allowResumeAction, turnId])

  return (
    <div className="space-y-2">
      <TurnCardUserBlock
        userText={input.view.userText}
        timestamp={undefined}
      />

      {!allowResumeAction && (input.sessionRuntimeState === 'degraded' || input.sessionRuntimeState === 'suspended_confirmation' || input.sessionRuntimeState === 'suspended_askuser') ? (
        <div className="rounded border border-warning/40 bg-warning/10 px-2.5 py-2 text-xs text-warning">
          {ai.panel.turnResumeUnavailableReadOnly}
        </div>
      ) : null}

      <div className="space-y-2 pl-0.5">
        {input.view.state.phase === 'failed' && input.view.state.turnError && (
          <TurnErrorCard
            descriptor={classifyTurnError(input.view.state.turnError)}
            onRetry={input.onRetryTurn}
            onOpenSettings={input.onOpenSettings}
          />
        )}

        {toolExposure ? (
          <div className="rounded border border-border/60 bg-muted/30 px-2.5 py-2 text-[11px] text-muted-foreground">
            <div className="font-medium text-foreground">
              {`package ${toolExposure.toolPackage ?? 'unknown'}`}
            </div>
            {toolExposure.routeReason ? <div>{`route ${toolExposure.routeReason}`}</div> : null}
            {toolExposure.fallbackFrom ? (
              <div>{`fallback ${toolExposure.fallbackFrom}${toolExposure.fallbackReason ? ` · ${toolExposure.fallbackReason}` : ''}`}</div>
            ) : null}
            {toolExposure.exposedTools.length > 0 ? (
              <div className="mt-1 break-all">
                {`tools ${toolExposure.exposedTools.join(', ')}`}
              </div>
            ) : null}
          </div>
        ) : null}

        {input.viewMode === 'debug' ? (
          <TimelineBlocksRenderer
            blocks={timeline.blocks}
            turn={input.view.state}
            toolSteps={input.view.toolSteps}
            sessionId={input.sessionId}
            running={activeTurn}
            viewMode={input.viewMode}
            onRetryStep={input.onRetryStep}
            onRetryTurn={handleRetry}
            onApprove={allowResumeAction ? handleApprove : undefined}
            onSkip={allowResumeAction ? handleSkip : undefined}
            pendingAskUser={allowResumeAction ? turnPendingAskUser : undefined}
            onResolveAskUser={input.onResolveAskUser}
            onCancelAskUser={input.onCancelAskUser}
            hideInlineLoadingIndicator={activeTurn && input.running}
          />
        ) : (
          <>
            <TurnCardContent
              text={stripThinkingPrefix(input.view.answerText ?? '', input.view.thinkingText ?? '')}
              turn={input.view.state}
              running={activeTurn}
              retryable={!activeTurn && input.view.state.phase === 'failed'}
              hideInlineLoadingIndicator={activeTurn && input.running}
              onRetry={input.onRetryTurn}
            />

            {toolCallCount > 0 || hasThinkingPanel ? (
              <div className="space-y-1.5">
                <TurnExecutionToggle
                  open={executionOpen}
                  disabled={forceOpenExecution}
                  running={activeTurn && input.running}
                  toolCount={toolCallCount}
                  hasThinking={hasThinkingPanel}
                  onToggle={handleToggleExecution}
                />

                {executionOpen ? (
                  <div className="space-y-2">
                    <TimelineBlocksRenderer
                      blocks={executionBlocks}
                      turn={input.view.state}
                      toolSteps={input.view.toolSteps}
                      sessionId={input.sessionId}
                      running={activeTurn}
                      viewMode={input.viewMode}
                      onRetryStep={input.onRetryStep}
                      onRetryTurn={handleRetry}
                      onApprove={allowResumeAction ? handleApprove : undefined}
                      onSkip={allowResumeAction ? handleSkip : undefined}
                      pendingAskUser={allowResumeAction ? turnPendingAskUser : undefined}
                      onResolveAskUser={input.onResolveAskUser}
                      onCancelAskUser={input.onCancelAskUser}
                      hideInlineLoadingIndicator={activeTurn && input.running}
                    />
                  </div>
                ) : null}
              </div>
            ) : null}
          </>
        )}

        <TurnExecutionTimeline
          phase={input.view.state.phase}
          stage={timeline.stage}
          running={activeTurn && input.running}
        />
      </div>
    </div>
  )
}
