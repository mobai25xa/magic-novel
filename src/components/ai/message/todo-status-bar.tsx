import { useMemo, useState } from 'react'
import { CheckCircle2, ChevronDown, ChevronRight, ListTodo } from 'lucide-react'

import type { AgentTodoItem, AgentTodoState } from '@/agent/types'

import { AiStatusShell, Button, Collapse } from '@/magic-ui/components'
import { useAiTranslations } from '../ai-hooks'

type TodoStatusBarProps = {
  todoState: AgentTodoState
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

function toStatusLabel(input: {
  ai: ReturnType<typeof useAiTranslations>
  status: AgentTodoItem['status']
}) {
  return input.ai.todo.status[input.status]
}

export function TodoStatusBar({ todoState }: TodoStatusBarProps) {
  const ai = useAiTranslations()
  const [collapsed, setCollapsed] = useState(true)

  const summary = useMemo(() => {
    const total = todoState.items.length
    const completed = todoState.items.filter((item) => item.status === 'completed').length
    const inProgressItem = todoState.items.find((item) => item.status === 'in_progress')
    const pendingItem = todoState.items.find((item) => item.status === 'pending')
    const fallback = todoState.items[0]

    const focusItem = inProgressItem || pendingItem || fallback

    return {
      total,
      completed,
      focusItem,
    }
  }, [todoState.items])

  if (summary.total === 0) {
    return null
  }

  return (
    <AiStatusShell className="mb-2">
      <Button
        type="button"
        variant="ghost"
        onClick={() => setCollapsed((value) => !value)}
        className="w-full h-auto px-3 py-2 justify-start text-left"
        aria-expanded={!collapsed}
        aria-label={collapsed ? ai.tool.ariaExpand : ai.tool.ariaCollapse}
      >
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-1.5 min-w-0">
            {collapsed ? (
              <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            ) : (
              <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            )}
            <ListTodo className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            <span className="text-xs font-medium text-foreground">{ai.todo.title}</span>
          </div>

          <div className="text-[11px] text-muted-foreground shrink-0">
            {summary.completed}/{summary.total}
          </div>
        </div>

        {summary.focusItem ? (
          <div className="mt-1.5 flex items-start gap-1.5 text-[11px] min-w-0">
            <span className="text-secondary-foreground shrink-0">[{toStatusLabel({ ai, status: summary.focusItem.status })}]</span>
            <span className="text-muted-foreground truncate">{summary.focusItem.text}</span>
          </div>
        ) : null}
      </Button>

      <Collapse
        collapsed={collapsed}
        onCollapsedChange={(next) => setCollapsed(next)}
        maxHeight={180}
      >
        <div className="border-t border-border px-3 py-2">
          <div className="text-[11px] text-muted-foreground mb-1.5">
            {ai.todo.updatedAtPrefix}: {formatTimestamp(todoState.lastUpdatedAt)}
          </div>

          <ol className="space-y-1.5 max-h-40 overflow-auto pr-1">
            {todoState.items.map((item, index) => (
              <li key={`${item.status}-${index}-${item.text.slice(0, 24)}`} className="text-xs flex items-start gap-2">
                <span className="text-muted-foreground w-5 shrink-0">{index + 1}.</span>
                <span className="text-secondary-foreground shrink-0">[{toStatusLabel({ ai, status: item.status })}]</span>
                <span className="text-foreground break-words">{item.text}</span>
                {item.status === 'completed' ? <CheckCircle2 className="h-3.5 w-3.5 text-ai-status-success mt-0.5 shrink-0" /> : null}
              </li>
            ))}
          </ol>
        </div>
      </Collapse>
    </AiStatusShell>
  )
}
