import type { Tokens } from 'marked'
import { cn } from '@/lib/utils'
import { Filespan } from '@/components/ai/design/Filespan'
import { InlineRenderer } from './InlineRenderer'

type LinkProps = {
  token: Tokens.Link
  className?: string
}

function isExternalLink(href: string): boolean {
  return /^https?:\/\//.test(href)
}

function isPathReference(href: string): boolean {
  return href.startsWith('manuscripts/') || href.startsWith('magic_assets/')
}

export function Link({ token, className }: LinkProps) {
  if (isPathReference(token.href)) {
    return <Filespan path={token.href} className={className} />
  }

  return (
    <a
      href={token.href}
      title={token.title ?? undefined}
      className={cn('text-primary underline hover:opacity-80 transition-opacity', className)}
      {...(isExternalLink(token.href) ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
    >
      <InlineRenderer tokens={token.tokens} />
    </a>
  )
}
