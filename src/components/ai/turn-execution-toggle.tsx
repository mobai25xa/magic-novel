import { ChevronDown, Wrench } from 'lucide-react'

import { cn } from '@/lib/utils'
import { Spinner } from '@/magic-ui/components'

import { useAiTranslations } from './ai-hooks'

export type TurnExecutionToggleProps = {
  open: boolean
  disabled?: boolean
  running?: boolean
  toolCount?: number
  hasThinking?: boolean
  onToggle: () => void
  className?: string
}

export function TurnExecutionToggle({
  open,
  disabled,
  running,
  toolCount,
  hasThinking,
  onToggle,
  className,
}: TurnExecutionToggleProps) {
  const ai = useAiTranslations()

  const labelParts: string[] = []
  if (typeof toolCount === 'number' && toolCount > 0) {
    labelParts.push(`${ai.turn.tools} (${toolCount})`)
  } else {
    labelParts.push(ai.turn.tools)
  }
  if (hasThinking) {
    labelParts.push(ai.turn.thinking)
  }

  return (
    <button
      type="button"
      className={cn(
        'flex w-full items-center justify-between gap-2 rounded-md border border-border/60 bg-muted/20 px-2.5 py-1.5 text-xs',
        'text-foreground transition-colors hover:bg-muted/30',
        'disabled:cursor-not-allowed disabled:opacity-60',
        className,
      )}
      onClick={onToggle}
      disabled={disabled}
      aria-expanded={open}
    >
      <span className="flex min-w-0 items-center gap-2">
        <span className="inline-flex w-4 items-center justify-center" aria-hidden="true">
          {running ? (
            <Spinner size="xs" className="text-ai-status-running" />
          ) : (
            <Wrench className="h-3.5 w-3.5 text-muted-foreground" />
          )}
        </span>
        <span className="truncate">{labelParts.join(' · ')}</span>
      </span>

      <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
        <span>{open ? ai.action.showLess : ai.action.showMore}</span>
        <ChevronDown
          className={cn('h-3.5 w-3.5 transition-transform', open && 'rotate-180')}
          aria-hidden="true"
        />
      </span>
    </button>
  )
}
