import { useMemo } from 'react'

import type { AgentTodoItem, AgentTodoState } from '@/agent/types'

import { AiStatusShell } from '@/magic-ui/components'

import { useAiTranslations } from '../ai-hooks'

type TodoPanelProps = {
  todoState: AgentTodoState
}

function toStatusLabel(input: {
  ai: ReturnType<typeof useAiTranslations>
  status: AgentTodoItem['status']
}) {
  return input.ai.todo.status[input.status]
}

function formatTimestamp(ts: number) {
  if (!Number.isFinite(ts) || ts <= 0) {
    return '--'
  }
  try {
    return new Date(ts).toLocaleString()
  } catch {
    return String(ts)
  }
}

export function TodoPanel({ todoState }: TodoPanelProps) {
  const ai = useAiTranslations()

  const sortedItems = useMemo(() => {
    return todoState.items.map((item, index) => ({ item, index }))
  }, [todoState.items])

  return (
    <AiStatusShell className="p-2.5 space-y-2">
      <div className="flex items-center justify-between gap-3">
        <div className="text-xs font-medium text-foreground">{ai.todo.title}</div>
        <div className="text-[11px] text-muted-foreground">
          {ai.todo.updatedAtPrefix}: {formatTimestamp(todoState.lastUpdatedAt)}
        </div>
      </div>

      {todoState.sourceCallId ? (
        <div className="text-[11px] text-muted-foreground break-all">
          {ai.todo.sourceCallPrefix}: {todoState.sourceCallId}
        </div>
      ) : null}

      {sortedItems.length === 0 ? (
        <div className="text-xs text-muted-foreground">{ai.todo.empty}</div>
      ) : (
        <ol className="space-y-1.5">
          {sortedItems.map(({ item, index }) => (
            <li key={`${item.status}-${index}-${item.text.slice(0, 24)}`} className="text-xs flex items-start gap-2">
              <span className="text-muted-foreground w-5 shrink-0">{index + 1}.</span>
              <span className="text-secondary-foreground shrink-0">[{toStatusLabel({ ai, status: item.status })}]</span>
              <span className="text-foreground break-words">{item.text}</span>
            </li>
          ))}
        </ol>
      )}
    </AiStatusShell>
  )
}
