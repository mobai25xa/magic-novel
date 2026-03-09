import { ChevronDown, ChevronRight } from 'lucide-react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { ToolCallStatus } from './ToolCallStatus'
import { ToolIcon } from './ToolIcon'
import { resolveToolIconName } from './tool-icon-map'
import { resolveStepArgsSummary, resolveStepDurationMs, formatDurationMs } from './tool-view-utils'

type ToolCallHeaderProps = {
  step: AgentUiToolStep
  now: number
  collapsed: boolean
}

export function ToolCallHeader({ step, now, collapsed }: ToolCallHeaderProps) {
  const iconName = resolveToolIconName(step.toolName)
  const secondaryInfo = resolveStepArgsSummary(step)
  const durationMs = resolveStepDurationMs(step, now)
  const durationLabel = formatDurationMs(durationMs)

  return (
    <div className="ai-tool-card-head-row flex flex-1 items-center gap-2 min-w-0">
      <ToolCallStatus status={step.status} progress={step.progress} dotOnly />

      <span className="ai-tool-card-icon-wrap" aria-hidden="true">
        <ToolIcon name={iconName} />
      </span>

      <span className="ai-tool-card-name">{step.toolName}</span>

      {secondaryInfo ? (
        <span className="ai-tool-card-args truncate min-w-0 flex-1">
          {secondaryInfo}
        </span>
      ) : (
        <span className="flex-1" />
      )}

      <span className="ai-tool-card-duration shrink-0">{durationLabel}</span>

      {collapsed ? (
        <ChevronRight className="ai-tool-card-chevron" aria-hidden="true" />
      ) : (
        <ChevronDown className="ai-tool-card-chevron" aria-hidden="true" />
      )}
    </div>
  )
}
