import type { Tokens } from 'marked'
import { cn } from '@/lib/utils'
import { DataTable } from '@/magic-ui/components'
import { InlineRenderer } from './InlineRenderer'

type TableProps = {
  token: Tokens.Table
  className?: string
}

export function Table({ token, className }: TableProps) {
  return (
    <DataTable containerClassName={cn('my-2', className)}>
      <thead>
        <tr>
          {token.header.map((cell, i) => (
            <th
              key={i}
              style={cell.align ? { textAlign: cell.align } : undefined}
            >
              <InlineRenderer tokens={cell.tokens} />
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {token.rows.map((row, ri) => (
          <tr key={ri}>
            {row.map((cell, ci) => (
              <td
                key={ci}
                style={cell.align ? { textAlign: cell.align } : undefined}
              >
                <InlineRenderer tokens={cell.tokens} />
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </DataTable>
  )
}
