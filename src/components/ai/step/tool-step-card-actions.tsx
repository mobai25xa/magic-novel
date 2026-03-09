import { reportUiCounter } from '../panel/agent-chat-panel-metrics'
import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../ai-hooks'
import { buildStepErrorText, copyStepPreview, resolveStepPath } from './tool-step-view-utils'
import { classifyToolError } from '../error/classify-error'
import { CATEGORY_ICONS, CATEGORY_COLORS } from '../error/error-ui-config'

type ToolStepCardActionsProps = {
  step: AgentUiToolStep
  sessionId: string
  turnId?: number
  running: boolean
  onRetryStep: (turnId: number, callId: string) => void
}

export function ToolStepCardActions(input: ToolStepCardActionsProps) {
  const ai = useAiTranslations()

  if (input.step.status !== 'error') {
    return null
  }

  const stepPath = resolveStepPath(input.step)
  const descriptor = classifyToolError(input.step)
  const Icon = CATEGORY_ICONS[descriptor.category]
  const colorClass = CATEGORY_COLORS[descriptor.category]

  return (
    <div className="mt-2 space-y-1">
      <div className="flex items-center gap-1.5 text-[11px]">
        <Icon className={`size-3.5 ${colorClass}`} />
        <span className={`${colorClass} font-medium`}>
          {ai.error?.categoryTitle?.[descriptor.category] ?? descriptor.category}
        </span>
      </div>
      <div className="flex items-center gap-2 text-[11px]">
        {input.step.retryable ? (
          <button
            type="button"
            className="rounded border px-2 py-0.5 hover-bg disabled:opacity-60"
            disabled={input.running || typeof input.turnId !== 'number'}
            onClick={() => {
              reportUiCounter({
                sessionId: input.sessionId,
                turnId: input.turnId,
                metric: 'step_retry_click_rate',
                tags: {
                  callId: input.step.callId,
                  toolName: input.step.toolName,
                },
              })

              if (typeof input.turnId === 'number') {
                input.onRetryStep(input.turnId, input.step.callId)
              }
            }}
          >
            {ai.action.retryStep}
          </button>
        ) : null}
        <button
          type="button"
          className="rounded border px-2 py-0.5 hover-bg"
          onClick={() => {
            void copyStepPreview(buildStepErrorText(input.step))
          }}
        >
          {ai.action.copyError}
        </button>
        {stepPath ? (
          <button
            type="button"
            className="rounded border px-2 py-0.5 hover-bg"
            onClick={() => {
              void copyStepPreview(stepPath)
            }}
          >
            {ai.action.jumpPath}
          </button>
        ) : null}
      </div>
    </div>
  )
}
