import type { Token } from 'marked'

import { renderInlineToken } from './inline-token-renderer'

type InlineRendererProps = {
  tokens: Token[]
}

function renderInlineTokens(tokens: Token[]) {
  return tokens.map((token, index) => renderInlineToken({
    token,
    index,
    renderTokens: renderInlineTokens,
  }))
}

export function InlineRenderer({ tokens }: InlineRendererProps) {
  return <>{renderInlineTokens(tokens)}</>
}
