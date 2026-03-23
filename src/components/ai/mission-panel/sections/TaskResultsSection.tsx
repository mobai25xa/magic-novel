import { AiStatusBadge } from '@/components/ai/status-badge'
import type { MissionResultEntry } from '@/lib/tauri-commands/mission'

type TaskResultsSectionProps = {
  taskResults: MissionResultEntry[]
  open: boolean
  onOpenChange: (open: boolean) => void
  openByKey: Record<string, boolean>
  onEntryOpenChange: (key: string, open: boolean) => void
}

type TaskResultEntryCardProps = {
  resultKey: string
  taskResult: MissionResultEntry
  ok: boolean
  issues: string[]
  artifacts: string[]
  evidence: string[]
  nextActions: string[]
  commandsRun: string[]
  open: boolean
  onEntryOpenChange: (key: string, open: boolean) => void
}

type TaskResultDetailListProps = {
  title: string
  items: unknown[]
  monospace?: boolean
}

function TaskResultDetailList({ title, items, monospace }: TaskResultDetailListProps) {
  if (items.length === 0) {
    return null
  }

  const listClassName = monospace
    ? 'mt-1 space-y-1 font-mono text-[11px] text-muted-foreground'
    : 'mt-1 space-y-1 text-muted-foreground'

  return (
    <div className="mt-2">
      <div className="text-[11px] font-medium text-secondary-foreground">{title}</div>
      <ul className={listClassName}>
        {items.map((item, idx) => (
          <li key={idx} className={monospace ? 'break-all' : 'break-words'}>{String(item)}</li>
        ))}
      </ul>
    </div>
  )
}

function TaskResultEntryCard({
  resultKey,
  taskResult,
  ok,
  issues,
  artifacts,
  evidence,
  nextActions,
  commandsRun,
  open,
  onEntryOpenChange,
}: TaskResultEntryCardProps) {
  const statusGlyph = ok
    ? '✓'
    : taskResult.status === 'blocked'
      ? '!'
      : taskResult.status === 'cancelled'
        ? '∅'
        : '✗'

  return (
    <details
      className="rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs"
      open={open}
      onToggle={(event) => onEntryOpenChange(resultKey, event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className={ok ? 'text-ai-status-success' : 'text-destructive'}>
                {statusGlyph}
              </span>
              <span className="font-mono text-muted-foreground truncate" title={taskResult.actor_id}>{taskResult.actor_id}</span>
              <span className="truncate opacity-80" title={taskResult.task_id}>{taskResult.task_id}</span>
            </div>
            <div className="mt-0.5 text-muted-foreground truncate" title={taskResult.summary}>
              {taskResult.summary}
            </div>
          </div>

          <AiStatusBadge
            status={taskResult.status}
            label={issues.length > 0 ? `issues ${issues.length}` : undefined}
          />
        </div>
      </summary>

      <TaskResultDetailList title="Issues" items={issues} />
      <TaskResultDetailList title="Artifacts" items={artifacts} monospace />
      <TaskResultDetailList title="Evidence" items={evidence} />
      <TaskResultDetailList title="Next Actions" items={nextActions} />
      <TaskResultDetailList title="Commands" items={commandsRun} monospace />
    </details>
  )
}

export function TaskResultsSection({ taskResults, open, onOpenChange, openByKey, onEntryOpenChange }: TaskResultsSectionProps) {
  if (taskResults.length === 0) {
    return null
  }

  return (
    <details
      className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
      open={open}
      onToggle={(event) => onOpenChange(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Task Results (${taskResults.length})`}
      </summary>

      <div className="mt-2 space-y-2 max-h-56 overflow-y-auto pr-1">
        {taskResults.map((taskResult) => {
          const issues = Array.isArray(taskResult.issues) ? taskResult.issues : []
          const artifacts = Array.isArray(taskResult.artifacts) ? taskResult.artifacts : []
          const evidence = Array.isArray(taskResult.evidence) ? taskResult.evidence : []
          const nextActions = Array.isArray(taskResult.next_actions) ? taskResult.next_actions : []
          const commandsRun = Array.isArray(taskResult.commands_run) ? taskResult.commands_run : []
          const ok = taskResult.status === 'completed'

          const defaultEntryOpen = !ok || issues.length > 0 || nextActions.length > 0
          const entryOpen = openByKey[taskResult.key] ?? defaultEntryOpen

          return (
            <TaskResultEntryCard
              key={taskResult.key}
              resultKey={taskResult.key}
              taskResult={taskResult}
              ok={ok}
              issues={issues}
              artifacts={artifacts}
              evidence={evidence}
              nextActions={nextActions}
              commandsRun={commandsRun}
              open={entryOpen}
              onEntryOpenChange={onEntryOpenChange}
            />
          )
        })}
      </div>
    </details>
  )
}
