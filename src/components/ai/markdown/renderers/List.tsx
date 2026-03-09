import type { Token, Tokens } from 'marked'
import { cn } from '@/lib/utils'
import { BlockTokenRenderer } from './BlockTokenRenderer'

type ListProps = {
  token: Tokens.List
  className?: string
}

export function List({ token, className }: ListProps) {
  const Tag = token.ordered ? 'ol' : 'ul'
  return (
    <Tag
      className={cn(
        'text-sm leading-relaxed ml-4 mb-2',
        token.ordered ? 'list-decimal' : 'list-disc',
        className,
      )}
      start={token.ordered && token.start !== '' ? token.start : undefined}
    >
      {token.items.map((item: Tokens.ListItem, i: number) => (
        <ListItem key={i} token={item} />
      ))}
    </Tag>
  )
}

function ListItem({ token }: { token: Tokens.ListItem }) {
  return (
    <li className="mb-0.5">
      {token.task && (
        <input
          type="checkbox"
          checked={token.checked ?? false}
          disabled
          className="mr-1.5 align-middle"
        />
      )}
      {token.tokens.map((child: Token, i: number) => (
        <BlockTokenRenderer key={i} token={child} inline />
      ))}
    </li>
  )
}
