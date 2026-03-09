import { useState, useCallback } from 'react'
import type { Tokens } from 'marked'
import { Copy, Check } from 'lucide-react'

import { Button } from '@/magic-ui/components'
import { cn } from '@/lib/utils'
import { ShowMore } from '@/magic-ui/components'

type CodeBlockProps = {
  token: Tokens.Code
  streaming?: boolean
  className?: string
}

const LINE_THRESHOLD = 10

export function CodeBlock({ token, streaming, className }: CodeBlockProps) {
  const [copied, setCopied] = useState(false)
  const lines = token.text.split('\n')
  const shouldTruncate = !streaming && lines.length > LINE_THRESHOLD

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(token.text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // clipboard API may fail in some environments
    }
  }, [token.text])

  const codeContent = (
    <pre className={cn(
      'code-block relative group',
      className,
    )}>
      {token.lang && (
        <span className="absolute top-1.5 left-3 text-[10px] text-muted-foreground select-none">
          {token.lang}
        </span>
      )}

      {!streaming && (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={handleCopy}
          className={cn(
            'absolute top-1.5 right-1.5 h-7 w-7',
            'opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer',
          )}
          title="Copy"
        >
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
        </Button>
      )}

      <code className={cn('block', token.lang && 'mt-4')}>
        {token.text}
        {streaming && <span className="ai-streaming-cursor" />}
      </code>
    </pre>
  )

  if (shouldTruncate) {
    return <ShowMore maxLines={LINE_THRESHOLD}>{codeContent}</ShowMore>
  }

  return codeContent
}
