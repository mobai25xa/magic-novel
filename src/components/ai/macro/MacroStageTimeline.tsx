/**
 * A6. MacroStageTimeline (P1)
 *
 * Lightweight horizontal timeline showing 5–6 stages per chapter:
 * context → draft → review → fix → writeback → integrate
 * Each node shows completed / in-progress / blocked / upcoming.
 * Pure display — no invoke / state management.
 */

import { cn } from '@/lib/utils'
import type { MacroStage } from './types'

const PIPELINE_STAGES: MacroStage[] = [
  'context',
  'draft',
  'review',
  'fix',
  'writeback',
  'integrate',
]

type NodeStatus = 'completed' | 'current' | 'blocked' | 'upcoming'

export type MacroStageTimelineProps = {
  currentStage?: MacroStage
  chapterStatus?: 'pending' | 'running' | 'completed' | 'blocked' | 'failed' | 'skipped'
  className?: string
}

function resolveNodeStatus(
  stage: MacroStage,
  currentStage: MacroStage | undefined,
  chapterStatus: MacroStageTimelineProps['chapterStatus'],
): NodeStatus {
  if (chapterStatus === 'completed') return 'completed'
  if (chapterStatus === 'pending' || !currentStage) return 'upcoming'

  const currentIdx = PIPELINE_STAGES.indexOf(currentStage)
  const stageIdx = PIPELINE_STAGES.indexOf(stage)

  // current stage not in pipeline (e.g. 'planning', 'cancelled') — treat all as upcoming
  if (currentIdx < 0) return 'upcoming'

  if (stageIdx < currentIdx) return 'completed'
  if (stageIdx === currentIdx) {
    if (chapterStatus === 'blocked' || chapterStatus === 'failed') return 'blocked'
    return 'current'
  }
  return 'upcoming'
}

const NODE_COLORS: Record<NodeStatus, string> = {
  completed: 'bg-emerald-500',
  current: 'bg-blue-500 ring-2 ring-blue-500/30',
  blocked: 'bg-amber-500 ring-2 ring-amber-500/30',
  upcoming: 'bg-muted-foreground/25',
}

const LINE_COLORS: Record<NodeStatus, string> = {
  completed: 'bg-emerald-500/60',
  current: 'bg-blue-500/40',
  blocked: 'bg-amber-500/40',
  upcoming: 'bg-muted-foreground/15',
}

const LABEL_COLORS: Record<NodeStatus, string> = {
  completed: 'text-emerald-600 dark:text-emerald-400',
  current: 'text-blue-600 dark:text-blue-400 font-medium',
  blocked: 'text-amber-600 dark:text-amber-400 font-medium',
  upcoming: 'text-muted-foreground',
}

function stageLabel(stage: MacroStage): string {
  return stage.charAt(0).toUpperCase() + stage.slice(1)
}

export function MacroStageTimeline({
  currentStage,
  chapterStatus,
  className,
}: MacroStageTimelineProps) {
  return (
    <div className={cn('flex items-start gap-0 overflow-x-auto', className)}>
      {PIPELINE_STAGES.map((stage, idx) => {
        const status = resolveNodeStatus(stage, currentStage, chapterStatus)
        const isLast = idx === PIPELINE_STAGES.length - 1

        return (
          <div key={stage} className="flex items-start">
            <div className="flex flex-col items-center gap-1">
              <div className={cn('h-2 w-2 rounded-full shrink-0', NODE_COLORS[status])} />
              <span className={cn('text-[10px] leading-tight', LABEL_COLORS[status])}>
                {stageLabel(stage)}
              </span>
            </div>
            {!isLast ? (
              <div
                className={cn(
                  'mt-[3px] h-[2px] w-4 shrink-0',
                  LINE_COLORS[
                    resolveNodeStatus(PIPELINE_STAGES[idx + 1], currentStage, chapterStatus) === 'upcoming'
                      ? 'upcoming'
                      : status
                  ],
                )}
              />
            ) : null}
          </div>
        )
      })}
    </div>
  )
}
