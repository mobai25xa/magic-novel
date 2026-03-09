import type { Tokens } from 'marked'

import { InlineRenderer } from './InlineRenderer'
import { cn } from '@/lib/utils'

const depthStyles: Record<number, string> = {
  1: 'text-lg font-bold mt-4 mb-2',
  2: 'text-base font-semibold mt-3 mb-1.5',
  3: 'text-sm font-medium mt-2 mb-1',
  4: 'text-sm font-medium mt-1.5 mb-1',
  5: 'text-sm font-medium mt-1.5 mb-1',
  6: 'text-sm font-medium mt-1.5 mb-1',
}

type HeadingProps = {
  token: Tokens.Heading
  className?: string
}

function resolveDepth(depth: number): 1 | 2 | 3 | 4 | 5 | 6 {
  if (depth <= 1) return 1
  if (depth >= 6) return 6
  return depth as 1 | 2 | 3 | 4 | 5
}

export function Heading({ token, className }: HeadingProps) {
  const depth = resolveDepth(token.depth)
  const content = <InlineRenderer tokens={token.tokens} />
  const classNames = cn(depthStyles[depth], className)

  switch (depth) {
    case 1:
      return <h1 className={classNames}>{content}</h1>
    case 2:
      return <h2 className={classNames}>{content}</h2>
    case 3:
      return <h3 className={classNames}>{content}</h3>
    case 4:
      return <h4 className={classNames}>{content}</h4>
    case 5:
      return <h5 className={classNames}>{content}</h5>
    default:
      return <h6 className={classNames}>{content}</h6>
  }
}
