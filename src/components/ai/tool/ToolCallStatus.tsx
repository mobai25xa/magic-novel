import { useState } from 'react'
import { Check, X, Ban } from 'lucide-react'

import { cn } from '@/lib/utils'
import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../ai-hooks'
import { Spinner } from '@/magic-ui/components'

type ToolCallStatusProps = {
  status: AgentUiToolStep['status']
  progress?: string
  cancellable?: boolean
  onCancel?: () => void
  dotOnly?: boolean
}

export function ToolCallStatus({ status, progress, cancellable, onCancel, dotOnly = false }: ToolCallStatusProps) {
  const ai = useAiTranslations()
  const [hovered, setHovered] = useState(false)

  if (status === 'running') {
    if (dotOnly) {
      return <span className="ai-tool-card-dot is-running" role="status" aria-label={ai.tool.statusLabel.running} />
    }

    if (cancellable && hovered) {
      return (
        <button
          type="button"
          className="flex items-center justify-center h-4 w-4 rounded-full text-ai-status-error hover:bg-error-15 transition-colors"
          style={{ transitionDuration: 'var(--ai-duration-fast)' }}
          onClick={onCancel}
          onMouseLeave={() => setHovered(false)}
          aria-label={ai.action.cancel}
        >
          <X className="h-3 w-3" />
        </button>
      )
    }

    return (
      <span
        className="flex items-center justify-center h-4 w-4"
        onMouseEnter={cancellable ? () => setHovered(true) : undefined}
        role="status"
        aria-label={ai.tool.statusLabel.running}
      >
        <Spinner size="xs" className="text-ai-status-running" />
      </span>
    )
  }

  if (status === 'waiting_confirmation') {
    const isAskUser = progress === 'waiting_askuser'
    const label = isAskUser ? ai.tool.statusLabel.waiting_askuser : ai.tool.statusLabel.waiting_confirmation

    if (dotOnly) {
      return <span className="ai-tool-card-dot is-waiting" role="status" aria-label={label} />
    }

    return (
      <span key={`waiting-${status}-${progress || 'default'}`} className="flex items-center justify-center h-4 w-4 ai-animate-scale-in" role="status" aria-label={label}>
        <span className="h-2.5 w-2.5 rounded-full ai-animate-pulse bg-ai-status-waiting" />
      </span>
    )
  }

  if (status === 'success') {
    if (dotOnly) {
      return <span className="ai-tool-card-dot is-success" role="status" aria-label={ai.tool.statusLabel.success} />
    }

    return (
      <span key={`success-${status}`} className="flex items-center justify-center h-4 w-4 text-ai-status-success ai-animate-scale-in" role="status" aria-label={ai.tool.statusLabel.success}>
        <Check className="h-3.5 w-3.5" />
      </span>
    )
  }

  if (status === 'error') {
    if (dotOnly) {
      return <span className="ai-tool-card-dot is-error" role="status" aria-label={ai.tool.statusLabel.error} />
    }

    return (
      <span key={`error-${status}`} className="flex items-center justify-center h-4 w-4 text-ai-status-error ai-animate-scale-in" role="status" aria-label={ai.tool.statusLabel.error}>
        <X className="h-3.5 w-3.5" />
      </span>
    )
  }

  if (dotOnly) {
    return <span className="ai-tool-card-dot is-cancelled" role="status" aria-label={ai.tool.statusLabel.cancelled} />
  }

  // cancelled
  return (
    <span key={`cancelled-${status}`} className={cn('flex items-center justify-center h-4 w-4 text-ai-status-cancelled ai-animate-scale-in')} role="status" aria-label={ai.tool.statusLabel.cancelled}>
      <Ban className="h-3.5 w-3.5" />
    </span>
  )
}
