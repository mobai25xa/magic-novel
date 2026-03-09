import { useEffect, useMemo, useState } from 'react'
import type { Token, Tokens } from 'marked'
import { cn } from '@/lib/utils'
import { parseMarkdown, isMermaidCodeBlock } from './markdown-parser'
import { BlockTokenRenderer } from './renderers/BlockTokenRenderer'
import { MermaidChart } from './MermaidChart'
import './markdown.css'

type MarkdownRendererProps = {
  text: string
  streaming?: boolean
  className?: string
}

const STREAMING_THROTTLE_MS = 50

function useMarkdownTokens(text: string, streaming?: boolean) {
  const immediateTokens = useMemo(() => {
    if (streaming || !text) {
      return [] as Token[]
    }
    return parseMarkdown(text)
  }, [streaming, text])

  const [tokens, setTokens] = useState<Token[]>(immediateTokens)

  useEffect(() => {
    if (!streaming) {
      queueMicrotask(() => {
        setTokens(immediateTokens)
      })
      return
    }

    if (!text) {
      queueMicrotask(() => {
        setTokens([])
      })
      return
    }

    // Streaming: throttle parsing to avoid blocking frames
    const timer = window.setTimeout(() => {
      setTokens(parseMarkdown(text))
    }, STREAMING_THROTTLE_MS)

    return () => window.clearTimeout(timer)
  }, [immediateTokens, streaming, text])

  return tokens
}

export function MarkdownRenderer({ text, streaming, className }: MarkdownRendererProps) {
  const tokens = useMarkdownTokens(text, streaming)

  if (tokens.length === 0) {
    return null
  }

  return (
    <div className={cn('ai-markdown', className)}>
      {tokens.map((token: Token, index: number) => {
        const isLast = index === tokens.length - 1

        // Mermaid code blocks get special treatment
        if (token.type === 'code' && isMermaidCodeBlock(token as Tokens.Code)) {
          return (
            <MermaidChart
              key={index}
              code={(token as Tokens.Code).text}
              streaming={isLast && streaming}
            />
          )
        }

        return (
          <BlockTokenRenderer
            key={index}
            token={token}
            streaming={streaming}
            isLastBlock={isLast}
          />
        )
      })}
    </div>
  )
}
