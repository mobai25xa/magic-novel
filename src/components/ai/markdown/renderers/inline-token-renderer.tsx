import type { Token, Tokens } from 'marked'

import { Filespan } from '@/components/ai/design/Filespan'

type RenderInlineTokenInput = {
  token: Token
  index: number
  renderTokens: (tokens: Token[]) => React.ReactNode
}

function isExternalLink(href: string): boolean {
  return /^https?:\/\//.test(href)
}

function isPathReference(href: string): boolean {
  return href.startsWith('manuscripts/') || href.startsWith('assets/')
}

function renderTextToken(token: Tokens.Text, index: number, renderTokens: (tokens: Token[]) => React.ReactNode) {
  if ('tokens' in token && token.tokens) {
    return <span key={index}>{renderTokens(token.tokens)}</span>
  }
  return <span key={index}>{token.text}</span>
}

function renderLinkToken(token: Tokens.Link, index: number, renderTokens: (tokens: Token[]) => React.ReactNode) {
  if (isPathReference(token.href)) {
    return <Filespan key={index} path={token.href} />
  }

  return (
    <a
      key={index}
      href={token.href}
      title={token.title ?? undefined}
      className="text-primary underline hover:opacity-80 transition-opacity"
      {...(isExternalLink(token.href) ? { target: '_blank', rel: 'noopener noreferrer' } : {})}
    >
      {renderTokens(token.tokens)}
    </a>
  )
}

function renderFallbackToken(token: Token, index: number) {
  if ('text' in token && typeof (token as { text: unknown }).text === 'string') {
    return <span key={index}>{(token as { text: string }).text}</span>
  }
  return null
}

export function renderInlineToken(input: RenderInlineTokenInput): React.ReactNode {
  const { token, index, renderTokens } = input

  switch (token.type) {
    case 'text':
      return renderTextToken(token as Tokens.Text, index, renderTokens)
    case 'strong':
      return <strong key={index} className="font-semibold">{renderTokens((token as Tokens.Strong).tokens)}</strong>
    case 'em':
      return <em key={index}>{renderTokens((token as Tokens.Em).tokens)}</em>
    case 'codespan':
      return <code key={index} className="rounded bg-secondary px-1 py-0.5 text-[0.9em]">{(token as Tokens.Codespan).text}</code>
    case 'link':
      return renderLinkToken(token as Tokens.Link, index, renderTokens)
    case 'image': {
      const imageToken = token as Tokens.Image
      return <img key={index} src={imageToken.href} alt={imageToken.text} title={imageToken.title ?? undefined} className="max-w-full rounded" loading="lazy" />
    }
    case 'del':
      return <del key={index} className="text-muted-foreground line-through">{renderTokens((token as Tokens.Del).tokens)}</del>
    case 'br':
      return <br key={index} />
    case 'escape':
      return <span key={index}>{(token as Tokens.Escape).text}</span>
    default:
      return renderFallbackToken(token, index)
  }
}
