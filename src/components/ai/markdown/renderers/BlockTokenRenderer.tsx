import type { Token, Tokens } from 'marked'
import { InlineRenderer } from './InlineRenderer'
import { Heading } from './Heading'
import { Paragraph } from './Paragraph'
import { List } from './List'
import { Table } from './Table'
import { HorizontalRule } from './HorizontalRule'
import { CodeBlock } from './CodeBlock'

type BlockTokenRendererProps = {
  token: Token
  streaming?: boolean
  isLastBlock?: boolean
  /** When true, render paragraph content inline (without <p> wrapper) */
  inline?: boolean
}

export function BlockTokenRenderer({ token, streaming, isLastBlock, inline }: BlockTokenRendererProps) {
  switch (token.type) {
    case 'heading':
      return <Heading token={token as Tokens.Heading} />

    case 'paragraph':
      if (inline) {
        return <InlineRenderer tokens={(token as Tokens.Paragraph).tokens} />
      }
      return <Paragraph token={token as Tokens.Paragraph} />

    case 'blockquote': {
      // Lazy import to avoid circular dependency at module level
      // Blockquote renders recursively via BlockTokenRenderer
      const bq = token as Tokens.Blockquote
      return (
        <blockquote className="border-l-2 border-l-[var(--ai-thinking-line)] pl-3 my-2 text-secondary-foreground italic">
          {bq.tokens.map((child: Token, i: number) => (
            <BlockTokenRenderer key={i} token={child} streaming={streaming} />
          ))}
        </blockquote>
      )
    }

    case 'list':
      return <List token={token as Tokens.List} />

    case 'table':
      return <Table token={token as Tokens.Table} />

    case 'code':
      return (
        <CodeBlock
          token={token as Tokens.Code}
          streaming={isLastBlock && streaming}
        />
      )

    case 'hr':
      return <HorizontalRule />

    case 'html': {
      const html = token as Tokens.HTML
      const sanitized = html.text
        .replace(/<script[\s>][\s\S]*?<\/script>/gi, '')
        .replace(/<iframe[\s>][\s\S]*?<\/iframe>/gi, '')
        .replace(/<object[\s>][\s\S]*?<\/object>/gi, '')
        .replace(/<embed[\s>][\s\S]*?>/gi, '')
        .replace(/<link[\s>][\s\S]*?>/gi, '')
        .replace(/\bon\w+\s*=\s*["'][^"']*["']/gi, '')
        .replace(/\bon\w+\s*=\s*\S+/gi, '')
        .replace(/javascript\s*:/gi, '')
      return <div className="my-2 text-sm" dangerouslySetInnerHTML={{ __html: sanitized }} />
    }

    case 'space':
      return null

    default:
      if ('tokens' in token && Array.isArray((token as { tokens: unknown }).tokens)) {
        return <InlineRenderer tokens={(token as { tokens: Token[] }).tokens} />
      }
      if ('text' in token && typeof (token as { text: unknown }).text === 'string') {
        return <span>{(token as { text: string }).text}</span>
      }
      return null
  }
}
