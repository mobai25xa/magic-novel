import type { Token, Tokens } from 'marked'
import { cn } from '@/lib/utils'
import { BlockTokenRenderer } from './BlockTokenRenderer'

type BlockquoteProps = {
  token: Tokens.Blockquote
  className?: string
}

export function Blockquote({ token, className }: BlockquoteProps) {
  return (
    <blockquote
      className={cn(
        'my-2 text-secondary-foreground italic',
        className,
      )}
    >
      {token.tokens.map((child: Token, i: number) => (
        <BlockTokenRenderer key={i} token={child} />
      ))}
    </blockquote>
  )
}
