import { useMemo, useState } from 'react'
import { ChevronDown } from 'lucide-react'

import type { AgentAskUserAnswer, AgentPendingAskUserRequest } from '@/agent/types'
import { resolveExecutionPhaseGroups, type ExecutionPhase } from '@/lib/agent-chat/execution-phases'
import type { TimelineBlock } from '@/lib/agent-chat/timeline'
import type { AgentUiTimelineEvent, AgentUiToolStep, AgentUiTurnState } from '@/lib/agent-chat/types'
import type { AiChatViewMode } from '@/state/settings'
import { cn } from '@/lib/utils'
import { Badge, Spinner } from '@/magic-ui/components'

import { useAiTranslations } from '../ai-hooks'
import { TimelineBlockThinkingPanel } from './TimelineBlockThinkingPanel'
import { TimelineBlockToolCall } from './TimelineBlockToolCall'

type ToolCallBlock = Extract<TimelineBlock, { type: 'tool_call' }>
type ThinkingPanelBlock = Extract<TimelineBlock, { type: 'thinking_panel' }>

type TimelineBlocksByPhaseRendererProps = {
  blocks: TimelineBlock[]
  events: AgentUiTimelineEvent[]
  turn: AgentUiTurnState
  toolSteps: AgentUiToolStep[]
  sessionId: string
  running: boolean
  viewMode: AiChatViewMode
  onRetryStep: (turnId: number, callId: string) => void
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
  pendingAskUser?: AgentPendingAskUserRequest
  onResolveAskUser: (callId: string, answers: AgentAskUserAnswer[]) => void
  onCancelAskUser: (callId: string) => void
}

function resolvePhaseLabel(ai: ReturnType<typeof useAiTranslations>, phase: ExecutionPhase) {
  const label = ai.turn.executionPhaseLabel?.[phase]
  return typeof label === 'string' && label.trim() ? label : phase
}

export function TimelineBlocksByPhaseRenderer(input: TimelineBlocksByPhaseRendererProps) {
  const ai = useAiTranslations()

  const stepByCallId = useMemo(
    () => new Map(input.toolSteps.map((step) => [step.callId, step] as const)),
    [input.toolSteps],
  )

  const thinkingBlocks = useMemo(
    () => input.blocks.filter((block): block is ThinkingPanelBlock => block.type === 'thinking_panel'),
    [input.blocks],
  )

  const toolCallBlocksByCallId = useMemo(() => {
    const map = new Map<string, ToolCallBlock>()
    input.blocks.forEach((block) => {
      if (block.type !== 'tool_call') {
        return
      }
      map.set(block.callId, block)
    })
    return map
  }, [input.blocks])

  const phaseGroups = useMemo(
    () => resolveExecutionPhaseGroups({ events: input.events, toolSteps: input.toolSteps, includeOrchestrator: true }),
    [input.events, input.toolSteps],
  )

  const currentGroupKey = useMemo(() => {
    if (!input.running || phaseGroups.length === 0) {
      return null
    }

    for (let index = phaseGroups.length - 1; index >= 0; index -= 1) {
      const group = phaseGroups[index]
      if (!group) continue
      if (typeof group.endedAt !== 'number') {
        return group.key
      }
    }

    return phaseGroups[phaseGroups.length - 1]?.key ?? null
  }, [input.running, phaseGroups])

  const [openByKey, setOpenByKey] = useState<Record<string, boolean>>({})

  return (
    <div className="space-y-2">
      {thinkingBlocks.map((block) => (
        <TimelineBlockThinkingPanel
          key={block.id}
          block={block}
          running={input.running}
        />
      ))}

      {phaseGroups.map((group) => {
        const phaseLabel = resolvePhaseLabel(ai, group.phase)
        const isCurrent = currentGroupKey === group.key
        const open = group.callIds.length > 0 ? (openByKey[group.key] ?? isCurrent) : false

        const groupHasError = group.callIds.some((callId) => stepByCallId.get(callId)?.status === 'error')

        if (group.callIds.length === 0) {
          return (
            <div
              key={group.key}
              className={cn(
                'rounded-md border border-border/60 bg-muted/10 px-2.5 py-2',
                isCurrent && 'bg-muted/20',
              )}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="flex min-w-0 items-center gap-2">
                  <span className="inline-flex w-4 items-center justify-center" aria-hidden="true">
                    {isCurrent ? (
                      <Spinner size="xs" className="text-ai-status-running" />
                    ) : (
                      <span className="text-muted-foreground">✓</span>
                    )}
                  </span>
                  <span className="truncate text-xs font-medium">
                    {phaseLabel}
                  </span>
                </span>
              </div>
            </div>
          )
        }

        return (
          <details
            key={group.key}
            className={cn(
              'rounded-md border border-border/60 bg-muted/10 px-2.5 py-2',
              isCurrent && 'bg-muted/20',
            )}
            open={open}
            onToggle={(event) => {
              const nextOpen = event.currentTarget.open
              setOpenByKey((prev) => ({
                ...prev,
                [group.key]: nextOpen,
              }))
            }}
          >
            <summary className="cursor-pointer select-none list-none">
              <div className="flex items-center justify-between gap-2">
                <span className="flex min-w-0 items-center gap-2">
                  <span className="inline-flex w-4 items-center justify-center" aria-hidden="true">
                    {isCurrent ? (
                      <Spinner size="xs" className="text-ai-status-running" />
                    ) : groupHasError ? (
                      <span className="text-ai-status-error">!</span>
                    ) : (
                      <span className="text-muted-foreground">✓</span>
                    )}
                  </span>
                  <span className={cn('truncate text-xs font-medium', groupHasError && 'text-ai-status-error')}>
                    {phaseLabel}
                  </span>
                  <Badge variant="outline" size="sm" className="shrink-0">
                    {group.callIds.length}
                  </Badge>
                </span>

                <ChevronDown
                  className={cn('h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform', open && 'rotate-180')}
                  aria-hidden="true"
                />
              </div>
            </summary>

            <div className="mt-2 space-y-1.5">
              {group.callIds.map((callId, index) => {
                const step = stepByCallId.get(callId)
                const block = toolCallBlocksByCallId.get(callId) ?? {
                  id: `tool_call_${callId}_${index}`,
                  type: 'tool_call',
                  seq: index,
                  ts: step?.startedAt ?? 0,
                  callId,
                }

                return (
                  <TimelineBlockToolCall
                    key={block.id}
                    block={block}
                    step={step}
                    turnId={input.turn.turn}
                    sessionId={input.sessionId}
                    running={input.running}
                    viewMode={input.viewMode}
                    onRetryStep={input.onRetryStep}
                    onApprove={input.onApprove}
                    onSkip={input.onSkip}
                    pendingAskUser={input.pendingAskUser}
                    onResolveAskUser={input.onResolveAskUser}
                    onCancelAskUser={input.onCancelAskUser}
                  />
                )
              })}
            </div>
          </details>
        )
      })}
    </div>
  )
}
