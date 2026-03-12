/**
 * A5. MacroWorkflowControls
 *
 * Start / Pause / Resume / Cancel / Refresh button shell.
 * Visibility driven by workflow state — callbacks provided by dev-b.
 * Pure display — no invoke / state management.
 */

import { cn } from '@/lib/utils'
import { Button } from '@/magic-ui/components'

export type MacroWorkflowPhase =
  | 'not_created'
  | 'awaiting_input'
  | 'running'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled'

export type MacroWorkflowControlsProps = {
  phase: MacroWorkflowPhase
  onStart?: () => void
  onPause?: () => void
  onResume?: () => void
  onCancel?: () => void
  onRefresh?: () => void
  disabled?: boolean
  className?: string
}

export function MacroWorkflowControls({
  phase,
  onStart,
  onPause,
  onResume,
  onCancel,
  onRefresh,
  disabled,
  className,
}: MacroWorkflowControlsProps) {
  const showStart = phase === 'not_created' || phase === 'awaiting_input'
  const showPause = phase === 'running'
  const showResume = phase === 'paused'
  const showCancel = phase === 'running' || phase === 'paused'
  const showRefresh = true

  return (
    <div className={cn('flex items-center gap-1.5', className)}>
      {showStart && onStart ? (
        <Button variant="default" size="sm" onClick={onStart} disabled={disabled}>
          Start
        </Button>
      ) : null}

      {showPause && onPause ? (
        <Button variant="outline" size="sm" onClick={onPause} disabled={disabled}>
          Pause
        </Button>
      ) : null}

      {showResume && onResume ? (
        <Button variant="outline" size="sm" onClick={onResume} disabled={disabled}>
          Resume
        </Button>
      ) : null}

      {showCancel && onCancel ? (
        <Button variant="destructive" size="sm" onClick={onCancel} disabled={disabled}>
          Cancel
        </Button>
      ) : null}

      {showRefresh && onRefresh ? (
        <Button variant="ghost" size="sm" onClick={onRefresh} disabled={disabled}>
          Refresh
        </Button>
      ) : null}
    </div>
  )
}
