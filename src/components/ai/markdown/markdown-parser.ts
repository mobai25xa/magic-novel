import { Marked, type Token, type Tokens } from 'marked'

const markedInstance = new Marked({
  breaks: true,
  gfm: true,
  async: false,
})

/** Parse Markdown text into a token AST */
export function parseMarkdown(text: string): Token[] {
  return markedInstance.lexer(text)
}

/** Check if a code token is a Mermaid block */
export function isMermaidCodeBlock(token: Tokens.Code): boolean {
  return token.lang?.toLowerCase() === 'mermaid'
}

/** Check if the text has an unclosed code fence (streaming state) */
export function isStreamingCodeBlock(text: string): boolean {
  const fenceCount = (text.match(/^```/gm) || []).length
  return fenceCount % 2 !== 0
}

/** Detect streaming state of the last block in the token list */
export function detectStreamingState(tokens: Token[]): {
  lastBlockStreaming: boolean
  lastBlockType: string | null
} {
  if (tokens.length === 0) {
    return { lastBlockStreaming: false, lastBlockType: null }
  }

  const last = tokens[tokens.length - 1]

  // Reconstruct minimal raw text to check for unclosed code fences
  if (last.type === 'code' && last.raw != null) {
    const fenceCount = (last.raw.match(/^```/gm) || []).length
    if (fenceCount % 2 !== 0) {
      return { lastBlockStreaming: true, lastBlockType: 'code' }
    }
  }

  return { lastBlockStreaming: false, lastBlockType: last.type }
}
