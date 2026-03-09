import { useCallback, useState } from 'react'
import { Copy, Check, RotateCcw } from 'lucide-react'

import { Button } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'

type MessageActionsProps = {
  text: string
  retryable?: boolean
  streaming?: boolean
  onRetry?: () => void
  className?: string
}

export function MessageActions({ text, retryable, streaming, onRetry, className }: MessageActionsProps) {
  const ai = useAiTranslations()
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 1200)
    } catch {
      // ignore clipboard errors
    }
  }, [text])

  if (streaming) {
    return null
  }

  return (
    <div
      className={cn(
        'editor-shell-ai-response-actions flex items-center gap-1 opacity-0 transition-opacity group-hover/assistant:opacity-100',
        'duration-[var(--ai-duration-fast)]',
        className,
      )}
    >
      <Button
        type="button"
        variant="ghost"
        size="icon"
        onClick={handleCopy}
        aria-label={copied ? ai.action.copied : ai.action.copy}
        className="editor-shell-ai-action-btn h-7 w-7"
      >
        {copied
          ? <Check className="h-3.5 w-3.5 text-[var(--ai-feedback-positive)]" />
          : <Copy className="h-3.5 w-3.5" />}
      </Button>

      {retryable ? (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={onRetry}
          aria-label={ai.action.retryStep}
          className="editor-shell-ai-action-btn h-7 w-7"
        >
          <RotateCcw className="h-3.5 w-3.5" />
        </Button>
      ) : null}
    </div>
  )
}
