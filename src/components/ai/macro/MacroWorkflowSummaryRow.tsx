/**
 * A1. MacroWorkflowSummaryRow
 *
 * One-line summary for the main chat stream: "Chapter 2/3 · review · blocked"
 * Pure display — no invoke / state management.
 */

import { cn } from '@/lib/utils'
import { Badge } from '@/magic-ui/components'
import type { BadgeProps } from '@/magic-ui/components'
import type { MacroStage } from './types'

export type MacroWorkflowSummaryRowProps = {
  objective?: string
  currentIndex: number
  total: number
  currentStage: MacroStage
  blocked?: boolean
  lastTransitionAt?: number
  className?: string
}

function stageColor(stage: MacroStage): BadgeProps['color'] {
  switch (stage) {
    case 'completed':
    case 'integrate':
      return 'success'
    case 'draft':
    case 'context':
    case 'planning':
      return 'info'
    case 'review':
    case 'fix':
    case 'writeback':
      return 'warning'
    case 'blocked':
    case 'failed':
    case 'cancelled':
      return 'error'
    default:
      return 'default'
  }
}

function formatTime(ts?: number) {
  if (!ts) return null
  try {
    const d = new Date(ts)
    return Number.isNaN(d.getTime()) ? null : d.toLocaleTimeString()
  } catch {
    return null
  }
}

export function MacroWorkflowSummaryRow({
  objective,
  currentIndex,
  total,
  currentStage,
  blocked,
  lastTransitionAt,
  className,
}: MacroWorkflowSummaryRowProps) {
  const chapterLabel = currentIndex < 0
    ? `0/${total}`
    : `${Math.min(currentIndex + 1, total)}/${total}`

  const time = formatTime(lastTransitionAt)

  return (
    <div
      className={cn(
        'flex items-center justify-between gap-2 rounded-md border border-border/60 bg-muted/20 px-2.5 py-1.5 text-xs',
        className,
      )}
    >
      <div className="flex min-w-0 items-center gap-2">
        <span className="font-medium truncate">{objective ?? 'Macro'}</span>
        <span className="text-muted-foreground">{`Chapter ${chapterLabel}`}</span>
        <Badge color={stageColor(currentStage)} variant="soft" size="sm">
          {currentStage}
        </Badge>
        {blocked ? (
          <Badge color="error" variant="outline" size="sm">blocked</Badge>
        ) : null}
      </div>

      {time ? (
        <span className="shrink-0 text-muted-foreground">{time}</span>
      ) : null}
    </div>
  )
}
