import { useState } from 'react'
import type { Tokens } from 'marked'
import { cn } from '@/lib/utils'

type ImageProps = {
  token: Tokens.Image
  className?: string
}

export function Image({ token, className }: ImageProps) {
  const [error, setError] = useState(false)

  if (error) {
    return (
      <span className={cn('inline-flex items-center gap-1 text-sm text-muted-foreground italic', className)}>
        [{token.text || 'Image failed to load'}]
      </span>
    )
  }

  return (
    <img
      src={token.href}
      alt={token.text}
      title={token.title ?? undefined}
      className={cn('max-w-full rounded my-2', className)}
      loading="lazy"
      onError={() => setError(true)}
    />
  )
}
