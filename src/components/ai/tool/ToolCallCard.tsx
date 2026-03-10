import { useState, useCallback } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import {
  AiToolCardHeaderButton,
  AiToolCardShell,
  AiToolContent,
  Button,
  Collapse,
} from '@/magic-ui/components'
import { useAiTranslations } from '../ai-hooks'
import { ToolCallHeader } from './ToolCallHeader'
import { ToolApprovalPanel } from './ToolApprovalPanel'
import { ToolViewDispatcher } from './ToolViewDispatcher'
import { useRunningStepClock } from './tool-call-hooks'
import { buildStepErrorText, copyStepPreview } from './tool-view-utils'

type ToolCallCardProps = {
  step: AgentUiToolStep
  turnId: number
  sessionId: string
  running: boolean
  viewMode?: 'compact' | 'debug'
  isLastAwaitingApproval?: boolean
  onRetryStep: (turnId: number, callId: string) => void
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
}

function renderDebugPanel(step: AgentUiToolStep, viewMode?: 'compact' | 'debug') {
  if (viewMode !== 'debug') {
    return null
  }

  const rows = [
    ['call_id', step.callId],
    ['llm_call_id', step.llmCallId],
    ['status', step.status],
    ['progress', step.progress],
    ['stage', step.stage],
    ['fault_domain', step.faultDomain],
    ['tx_id', step.txId],
  ].filter(([, value]) => typeof value === 'string' && value)

  if (rows.length === 0) {
    return null
  }

  return (
    <AiToolContent className="space-y-1 text-[11px] text-muted-foreground">
      {rows.map(([label, value]) => (
        <div key={label} className="break-all">
          <span className="font-medium text-foreground/80">{label}</span>
          {`: ${value}`}
        </div>
      ))}
    </AiToolContent>
  )
}

function useToolCardNow(step: AgentUiToolStep) {
  const stepRunning = step.status === 'running' || step.status === 'waiting_confirmation'
  const nowFromHook = useRunningStepClock(stepRunning)
  return stepRunning ? nowFromHook : (step.finishedAt ?? step.startedAt)
}

function useToolCardActions(input: {
  step: AgentUiToolStep
  turnId: number
  onRetryStep: (turnId: number, callId: string) => void
}) {
  const shouldAutoExpand = input.step.status === 'waiting_confirmation'
    && input.step.progress === 'waiting_confirmation'
  const [collapsedByUser, setCollapsedByUser] = useState(() => !shouldAutoExpand)
  const collapsed = shouldAutoExpand ? false : collapsedByUser

  const handleToggle = useCallback((next?: boolean) => {
    if (shouldAutoExpand) {
      return
    }
    setCollapsedByUser((value) => next ?? !value)
  }, [shouldAutoExpand])

  const handleRetry = useCallback(() => {
    input.onRetryStep(input.turnId, input.step.callId)
  }, [input])

  const handleCopyError = useCallback(() => {
    void copyStepPreview(buildStepErrorText(input.step))
  }, [input.step])

  return {
    collapsed,
    handleToggle,
    handleRetry,
    handleCopyError,
  }
}

function renderToolErrorPanel(input: {
  ai: ReturnType<typeof useAiTranslations>
  step: AgentUiToolStep
  running: boolean
  onRetry: () => void
  onCopyError: () => void
}) {
  if (input.step.status !== 'error') {
    return null
  }

  return (
    <AiToolContent className="space-y-1.5">
      <div className="ai-tool-card-error text-xs" role="alert">
        {input.step.errorMessage || input.step.errorCode || input.ai.tool.statusCopy.failed}
      </div>
      <div className="flex items-center gap-2">
        {input.step.retryable ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="ai-tool-card-action-btn h-auto rounded px-2 py-0.5 text-[11px]"
            disabled={input.running}
            onClick={input.onRetry}
          >
            {input.ai.action.retryStep}
          </Button>
        ) : null}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="ai-tool-card-action-btn h-auto rounded px-2 py-0.5 text-[11px]"
          onClick={input.onCopyError}
        >
          {input.ai.action.copyError}
        </Button>
      </div>
    </AiToolContent>
  )
}

function renderApprovalPanel(input: {
  step: AgentUiToolStep
  isLastAwaitingApproval?: boolean
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
}) {
  const showApproval = input.step.status === 'waiting_confirmation'
    && input.step.progress !== 'waiting_askuser'
    && input.isLastAwaitingApproval
    && input.onApprove
    && input.onSkip

  if (!showApproval) {
    return null
  }

  return (
    <ToolApprovalPanel
      callId={input.step.callId}
      toolName={input.step.toolName}
      visible
      onApprove={input.onApprove!}
      onSkip={input.onSkip!}
    />
  )
}

export function ToolCallCard({
  step,
  turnId,
  sessionId,
  running,
  viewMode,
  isLastAwaitingApproval,
  onRetryStep,
  onApprove,
  onSkip,
}: ToolCallCardProps) {
  const ai = useAiTranslations()
  const now = useToolCardNow(step)
  const actions = useToolCardActions({ step, turnId, onRetryStep })

  const collapsed = actions.collapsed

  const header = (
    <ToolCallHeader
      step={step}
      now={now}
      collapsed={collapsed}
    />
  )

  const expandedContent = (
    <>
      <ToolViewDispatcher
        step={step}
        sessionId={sessionId}
        turnId={turnId}
      />

      {renderDebugPanel(step, viewMode)}

      {renderToolErrorPanel({
        ai,
        step,
        running,
        onRetry: actions.handleRetry,
        onCopyError: actions.handleCopyError,
      })}

      {renderApprovalPanel({
        step,
        isLastAwaitingApproval,
        onApprove,
        onSkip,
      })}
    </>
  )

  return (
    <AiToolCardShell className="ai-animate-fade-in ai-transition-transform">
      <AiToolCardHeaderButton
        className="ai-tool-card-header-btn"
        onClick={() => actions.handleToggle()}
        aria-expanded={!collapsed}
        aria-label={collapsed ? ai.tool.ariaExpand : ai.tool.ariaCollapse}
      >
        {header}
      </AiToolCardHeaderButton>
      <Collapse
        collapsed={collapsed}
        onCollapsedChange={(next) => actions.handleToggle(next)}
        maxHeight={420}
      >
        <div className="ai-tool-card-body">{expandedContent}</div>
      </Collapse>
    </AiToolCardShell>
  )
}
