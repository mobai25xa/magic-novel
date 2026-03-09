import type { Tokens } from 'marked'
import { InlineRenderer } from './InlineRenderer'
import { cn } from '@/lib/utils'

type ParagraphProps = {
  token: Tokens.Paragraph
  className?: string
}

export function Paragraph({ token, className }: ParagraphProps) {
  return (
    <p className={cn('text-sm leading-relaxed mb-2', className)}>
      <InlineRenderer tokens={token.tokens} />
    </p>
  )
}
