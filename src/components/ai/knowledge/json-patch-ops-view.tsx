import { useMemo } from 'react'

import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge, ShowMore } from '@/magic-ui/components'

import { CopyPill } from './copy-pill'

export type JsonPatchOpLike = Record<string, unknown> & {
  op?: string
  path?: string
  from?: string
  value?: unknown
}

export type JsonPatchOpsViewProps = {
  ops: unknown
  maxOps?: number
  className?: string
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== 'object') {
    return false
  }
  const proto = Object.getPrototypeOf(value)
  return proto === Object.prototype || proto === null
}

function safeStringify(value: unknown) {
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function resolveOpLabel(op: JsonPatchOpLike): string {
  const raw = (typeof op.op === 'string' ? op.op : null)
    ?? (typeof op.kind === 'string' ? String(op.kind) : null)
    ?? (typeof op.type === 'string' ? String(op.type) : null)
  return raw ? String(raw) : 'op'
}

function resolveOpColor(label: string): BadgeProps['color'] {
  const normalized = String(label ?? '').trim().toLowerCase()
  if (normalized === 'add' || normalized === 'create') return 'success'
  if (normalized === 'remove' || normalized === 'delete') return 'error'
  if (normalized === 'replace' || normalized === 'update') return 'info'
  if (normalized === 'move') return 'warning'
  if (normalized === 'copy') return 'info'
  if (normalized === 'test') return 'default'
  return 'default'
}

export function JsonPatchOpsView({ ops, maxOps = 80, className }: JsonPatchOpsViewProps) {
  const rawEntries = useMemo(() => {
    if (!Array.isArray(ops)) {
      return []
    }
    return ops
      .filter((entry): entry is JsonPatchOpLike => isPlainObject(entry))
  }, [ops])

  const entries = rawEntries.slice(0, Math.max(0, maxOps))
  const truncated = rawEntries.length > entries.length

  if (rawEntries.length === 0) {
    return <div className={cn('text-[11px] text-muted-foreground', className)}>No patch ops.</div>
  }

  return (
    <div className={cn('space-y-2', className)}>
      <div className="flex flex-wrap items-center gap-2">
        <Badge color="info" variant="soft" size="sm">
          {`patch ${rawEntries.length}`}
        </Badge>
        {truncated ? (
          <span className="text-[11px] text-muted-foreground">{`showing ${entries.length}/${rawEntries.length}`}</span>
        ) : null}
      </div>

      <div className="space-y-2">
        {entries.map((op, idx) => {
          const label = resolveOpLabel(op)
          const path = typeof op.path === 'string' ? op.path : null
          const from = typeof op.from === 'string' ? op.from : null
          const valueText = op.value === undefined ? null : safeStringify(op.value)
          const opText = safeStringify(op)

          return (
            <div
              key={`${label}:${path ?? ''}:${from ?? ''}:${idx}`}
              className="rounded-md border border-border/60 bg-muted/10 px-2.5 py-2 text-xs"
            >
              <div className="flex flex-wrap items-center gap-2">
                <Badge color={resolveOpColor(label)} variant="soft" size="sm" className="font-mono">
                  {label}
                </Badge>
                {path ? <CopyPill value={path} title="Copy path" /> : null}
                {from ? <CopyPill value={from} title="Copy from" /> : null}
              </div>

              {valueText ? (
                <div className="mt-2">
                  <div className="text-[11px] font-medium text-secondary-foreground">Value</div>
                  <ShowMore maxLines={8} className="mt-1">
                    <pre className="whitespace-pre-wrap break-words font-mono text-[11px] text-muted-foreground">
                      {valueText}
                    </pre>
                  </ShowMore>
                </div>
              ) : null}

              <div className="mt-2">
                <div className="text-[11px] font-medium text-secondary-foreground">Op</div>
                <ShowMore maxLines={8} className="mt-1">
                  <pre className="whitespace-pre-wrap break-words font-mono text-[11px] text-muted-foreground">
                    {opText}
                  </pre>
                </ShowMore>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
