import { useCallback, useState } from 'react'

import { Button } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'

type ToolApprovalPanelProps = {
  callId: string
  toolName: string
  visible: boolean
  onApprove: (callId: string) => void
  onSkip: (callId: string) => void
}

function resolveApprovalMessage(
  ai: ReturnType<typeof useAiTranslations>,
  toolName: string,
): string {
  if (toolName === 'create') return ai.approval.create
  if (toolName === 'delete' || toolName === 'move') return ai.approval.edit
  if (toolName === 'edit') return ai.approval.edit
  return ai.approval.fallback
}

export function ToolApprovalPanel({ callId, toolName, visible, onApprove, onSkip }: ToolApprovalPanelProps) {
  const ai = useAiTranslations()
  const [approving, setApproving] = useState(false)

  const message = resolveApprovalMessage(ai, toolName)

  const handleApprove = useCallback(() => {
    setApproving(true)
    onApprove(callId)
  }, [callId, onApprove])

  const handleSkip = useCallback(() => {
    onSkip(callId)
  }, [callId, onSkip])

  if (!visible) return null

  return (
    <div
      role="alertdialog"
      aria-label={message}
      className={cn(
        'border-t border-ai-approval-border bg-ai-approval-bg px-3 py-2.5',
        'ai-animate-slide-in',
      )}
    >
      <p className="text-xs text-secondary-foreground mb-2">{message}</p>
      <div className="flex items-center justify-end gap-2">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={cn('text-xs')}
          style={{ transitionDuration: 'var(--ai-duration-fast)' }}
          onClick={handleSkip}
        >
          {ai.action.skip}
        </Button>

        <Button
          type="button"
          size="sm"
          className={cn(
            'text-xs hover:opacity-90 transition-opacity flex items-center gap-1.5',
            approving && 'opacity-70 pointer-events-none',
          )}
          style={{ transitionDuration: 'var(--ai-duration-fast)' }}
          onClick={handleApprove}
          disabled={approving}
        >
          <span>{ai.action.approve}</span>
          {!approving ? (
            <span className="h-2 w-2 rounded-full bg-white/70 ai-animate-pulse" aria-hidden="true" />
          ) : null}
        </Button>
      </div>
    </div>
  )
}
