import { Button } from '@/magic-ui/components'

type MissionActionButtonsProps = {
  canStart: boolean
  canResume: boolean
  canRecover?: boolean
  canPause: boolean
  canStop: boolean
  canAbandon: boolean
  resumeLabel?: string | null
  recoverLabel?: string | null
  stopLabel?: string | null
  loading: boolean
  onStart: () => void
  onPause: () => void
  onResume: () => void
  onRecover?: () => void
  onStop: () => void
  onAbandon: () => void
}

export function MissionActionButtons({
  canStart,
  canResume,
  canRecover = false,
  canPause,
  canStop,
  canAbandon,
  resumeLabel,
  recoverLabel,
  stopLabel,
  loading,
  onStart,
  onPause,
  onResume,
  onRecover,
  onStop,
  onAbandon,
}: MissionActionButtonsProps) {
  return (
    <div className="flex gap-2">
      {canStart && (
        <Button
          className="flex-1 text-xs font-medium disabled:opacity-50 hover:opacity-90"
          size="sm"
          onClick={onStart}
          disabled={loading}
        >
          Start
        </Button>
      )}

      {canResume && (
        <Button
          className="flex-1 text-xs font-medium disabled:opacity-50 hover:opacity-90"
          size="sm"
          onClick={onResume}
          disabled={loading}
        >
          {resumeLabel || 'Resume'}
        </Button>
      )}

      {canRecover && onRecover && (
        <Button
          variant="outline"
          className="flex-1 text-xs font-medium disabled:opacity-50"
          size="sm"
          onClick={onRecover}
          disabled={loading}
        >
          {recoverLabel || 'Recover'}
        </Button>
      )}

      {canPause && (
        <Button
          variant="outline"
          className="flex-1 text-xs font-medium disabled:opacity-50"
          size="sm"
          onClick={onPause}
          disabled={loading}
        >
          Pause
        </Button>
      )}

      {canStop && (
        <Button
          variant="outline"
          className="flex-1 text-xs font-medium disabled:opacity-50"
          size="sm"
          onClick={onStop}
          disabled={loading}
        >
          {stopLabel || 'Interrupt'}
        </Button>
      )}

      {canAbandon && (
        <Button
          variant="destructive"
          className="flex-1 text-xs font-medium disabled:opacity-50"
          size="sm"
          onClick={onAbandon}
          disabled={loading}
        >
          Abandon
        </Button>
      )}
    </div>
  )
}
