import { useCallback, useState } from 'react'
import { Check, Copy } from 'lucide-react'

import { cn } from '@/lib/utils'

export type CopyPillProps = {
  value: string
  title?: string
  className?: string
}

export function CopyPill({ value, title = 'Copy', className }: CopyPillProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1200)
    } catch {
      // ignore clipboard failures
    }
  }, [value])

  return (
    <button
      type="button"
      onClick={() => { void handleCopy() }}
      className={cn(
        'group inline-flex max-w-full items-center gap-1.5 rounded border border-border/60 bg-background px-2 py-1',
        'font-mono text-[11px] text-muted-foreground hover:text-foreground hover:bg-muted/30 transition-colors',
        className,
      )}
      title={title}
    >
      <span className="truncate">{value}</span>
      {copied ? (
        <Check className="h-3 w-3 shrink-0 opacity-70" />
      ) : (
        <Copy className="h-3 w-3 shrink-0 opacity-50 group-hover:opacity-80" />
      )}
    </button>
  )
}
