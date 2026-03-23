import { Clock3 } from 'lucide-react'
import { useMemo } from 'react'

import type { AgentTodoState } from '@/agent/types'
import { useAgentChatStore } from '@/state/agent'
import type { AgentUiTimelineEvent, AgentUiToolStep } from '@/lib/agent-chat/types'
import {
  resolveExecutionPhaseFromToolName,
  resolveExecutionPhaseFromWorkerType,
  type ExecutionPhase,
} from '@/lib/agent-chat/execution-phases'

import type { AgentChatPanelViewProps } from '../panel/view/agent-chat-panel-view-types'
import { handleToggleRun } from '../panel/view/agent-chat-panel-view-toggle-run'
import { useAiTranslations } from '../ai-hooks'
import { ChatInput } from '../input/ChatInput'
import { ActionBar } from '../input/ActionBar'
import { TodoStatusBar } from './todo-status-bar'

type AgentChatPanelInputShellProps = Pick<
  AgentChatPanelViewProps,
  | 'running'
  | 'input'
  | 'inputDisabled'
  | 'inputPlaceholder'
  | 'sessionCanContinue'
  | 'onInputChange'
  | 'onSend'
  | 'onCancel'
  | 'models'
  | 'selectedModel'
  | 'onSelectModel'
  | 'approvalMode'
  | 'elapsedTime'
  | 'elapsedSeconds'
  | 'showTimer'
> & {
  todoState: AgentTodoState
}

type ActiveWorkerHint = {
  phase: ExecutionPhase
  objectHint?: string
}

function asRecord(input: unknown): Record<string, unknown> | null {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return null
  }

  return input as Record<string, unknown>
}

function asText(input: unknown): string | undefined {
  if (typeof input !== 'string') {
    return undefined
  }

  const value = input.trim()
  return value ? value : undefined
}

function shrinkHint(value: string, maxLength: number) {
  const normalized = value.trim()
  if (normalized.length <= maxLength) {
    return normalized
  }

  return `…${normalized.slice(normalized.length - (maxLength - 1))}`
}

function resolveActiveWorkerHint(events: AgentUiTimelineEvent[]): ActiveWorkerHint | null {
  const completedWorkerSessions = new Set<string>()
  const completedByPhase = new Map<ExecutionPhase, number>()

  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index]
    if (!event) {
      continue
    }

    if (event.type === 'WORKER_COMPLETED') {
      const meta = asRecord(event.meta)
      const phase = resolveExecutionPhaseFromWorkerType(meta?.worker_type ?? meta?.workerType)
      const workerSessionId = asText(meta?.worker_session_id)

      if (workerSessionId) {
        completedWorkerSessions.add(workerSessionId)
        continue
      }

      completedByPhase.set(phase, (completedByPhase.get(phase) ?? 0) + 1)
      continue
    }

    if (event.type !== 'WORKER_STARTED') {
      continue
    }

    const meta = asRecord(event.meta)
    const phase = resolveExecutionPhaseFromWorkerType(meta?.worker_type ?? meta?.workerType)
    const workerSessionId = asText(meta?.worker_session_id)

    if (workerSessionId) {
      if (completedWorkerSessions.has(workerSessionId)) {
        completedWorkerSessions.delete(workerSessionId)
        continue
      }
    } else {
      const completedCount = completedByPhase.get(phase) ?? 0
      if (completedCount > 0) {
        completedByPhase.set(phase, completedCount - 1)
        continue
      }
    }

    const summary = asText(meta?.summary) ?? asText(event.summary)
    const scopeRef = asText(meta?.scope_ref)
    const targetRef = asText(meta?.target_ref)

    return {
      phase,
      objectHint: summary ?? scopeRef ?? targetRef,
    }
  }

  return null
}

function resolveActiveToolPhase(toolSteps: AgentUiToolStep[]): ExecutionPhase | null {
  let active: AgentUiToolStep | null = null

  for (const step of toolSteps) {
    if (step.status !== 'running' && step.status !== 'waiting_confirmation') {
      continue
    }

    if (!active || step.startedAt >= active.startedAt) {
      active = step
    }
  }

  if (!active) {
    return null
  }

  return resolveExecutionPhaseFromToolName(active.toolName)
}

export function AgentChatPanelInputShell(input: AgentChatPanelInputShellProps) {
  const ai = useAiTranslations()

  const progressKey = useAgentChatStore((state) => {
    const turnId = state.turnOrder.at(-1)
    if (!turnId) {
      return null
    }

    const events = state.eventsByTurnId[turnId] ?? []
    const toolSteps = state.stepsByTurnId[turnId] ?? []

    const worker = resolveActiveWorkerHint(events)
    if (worker) {
      return `${worker.phase}|${worker.objectHint ?? ''}`
    }

    const phase = resolveActiveToolPhase(toolSteps)
    if (!phase) {
      return null
    }

    return `${phase}|`
  })

  const progress = useMemo(() => {
    if (!progressKey) {
      return null
    }

    const [phase, hint = ''] = progressKey.split('|', 2)
    const normalizedHint = hint.trim()

    return {
      phase: phase as ExecutionPhase,
      objectHint: normalizedHint ? shrinkHint(normalizedHint, 56) : undefined,
    }
  }, [progressKey])

  const showLongTaskHint = input.showTimer
    && typeof input.elapsedSeconds === 'number'
    && Number.isFinite(input.elapsedSeconds)
    && input.elapsedSeconds >= 30
    && Boolean(progress)

  const phaseLabel = progress ? (ai.turn.executionPhaseLabel?.[progress.phase] || progress.phase) : null
  const runtimeHint = input.showTimer && input.elapsedTime
    ? showLongTaskHint && phaseLabel
      ? `${ai.panel.generating} · ${input.elapsedTime} · ${ai.panel.workingOn}${phaseLabel}${progress?.objectHint ? `（${progress.objectHint}）` : ''}`
      : `${ai.panel.generating} · ${input.elapsedTime}`
    : null

  return (
    <div className="editor-shell-ai-input-wrap">
      {runtimeHint ? (
        <div className="flex min-w-0 items-center gap-1.5 px-3 py-1.5 mb-1.5 text-xs text-muted-foreground ai-animate-pulse chat-input-runtime-hint">
          <Clock3 className="h-3 w-3" />
          <span className="truncate">{runtimeHint}</span>
        </div>
      ) : null}

      <TodoStatusBar todoState={input.todoState} />

      <div className="chat-input-shell" data-disabled={input.inputDisabled ? 'true' : 'false'}>
        <ChatInput
          value={input.input}
          onChange={input.onInputChange}
          onSend={() => { void input.onSend() }}
          disabled={input.inputDisabled}
          placeholder={input.inputPlaceholder || ai.panel.inputPlaceholder}
        />

        <ActionBar
          models={input.models}
          selectedModel={input.selectedModel}
          onSelectModel={input.onSelectModel}
          approvalMode={input.approvalMode}
          disabled={input.inputDisabled}
          modelsDisabled={input.running}
          running={input.running}
          inputEmpty={!input.input.trim()}
          canContinue={input.sessionCanContinue}
          onToggleRun={() => handleToggleRun(input)}
        />
      </div>
    </div>
  )
}
