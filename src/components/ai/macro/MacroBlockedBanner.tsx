/**
 * A4. MacroBlockedBanner
 *
 * Blocked reason + recommended next-step buttons.
 * Button callbacks provided by dev-b (wiring layer).
 * Pure display — no invoke / state management.
 */

import { cn } from '@/lib/utils'
import { Button } from '@/magic-ui/components'

export type MacroBlockedBannerProps = {
  lastError?: string | { code?: string; message?: string }
  pendingDecision?: string
  onFix?: () => void
  onRetry?: () => void
  onSkip?: () => void
  className?: string
}

function resolveMessage(
  lastError?: MacroBlockedBannerProps['lastError'],
  pendingDecision?: string,
): string {
  if (pendingDecision) return pendingDecision
  if (!lastError) return 'Blocked — awaiting action.'
  if (typeof lastError === 'string') return lastError
  return lastError.message || lastError.code || 'Unknown error'
}

export function MacroBlockedBanner({
  lastError,
  pendingDecision,
  onFix,
  onRetry,
  onSkip,
  className,
}: MacroBlockedBannerProps) {
  const message = resolveMessage(lastError, pendingDecision)
  const hasActions = onFix || onRetry || onSkip

  return (
    <div
      className={cn(
        'flex flex-col gap-2 rounded-md border border-warning/40 bg-warning/5 px-2.5 py-2 text-xs',
        className,
      )}
      role="alert"
    >
      <p className="break-words">{message}</p>

      {hasActions ? (
        <div className="flex items-center gap-1.5">
          {onFix ? (
            <Button variant="outline" size="sm" onClick={onFix}>Fix</Button>
          ) : null}
          {onRetry ? (
            <Button variant="outline" size="sm" onClick={onRetry}>Retry</Button>
          ) : null}
          {onSkip ? (
            <Button variant="ghost" size="sm" onClick={onSkip}>Skip</Button>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
