import { useMemo } from 'react'

import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge, ShowMore } from '@/magic-ui/components'

import { CopyPill } from './copy-pill'

type DiffKind = 'added' | 'removed' | 'changed'

type DiffEntry = {
  path: string
  kind: DiffKind
  before?: unknown
  after?: unknown
}

export type JsonDiffViewProps = {
  before: unknown
  after: unknown
  maxEntries?: number
  className?: string
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== 'object') {
    return false
  }
  const proto = Object.getPrototypeOf(value)
  return proto === Object.prototype || proto === null
}

function safeStableStringify(value: unknown): string {
  const seen = new WeakSet<object>()

  const normalize = (input: unknown): unknown => {
    if (!input || typeof input !== 'object') {
      return input
    }

    if (seen.has(input as object)) {
      return '[Circular]'
    }
    seen.add(input as object)

    if (Array.isArray(input)) {
      return input.map((v) => normalize(v))
    }

    if (!isPlainObject(input)) {
      return String(input)
    }

    const entries = Object.entries(input)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => [k, normalize(v)])

    return Object.fromEntries(entries)
  }

  try {
    return JSON.stringify(normalize(value), null, 2)
  } catch {
    try {
      return JSON.stringify(value, null, 2)
    } catch {
      return String(value)
    }
  }
}

function isDeepEqual(a: unknown, b: unknown) {
  if (Object.is(a, b)) {
    return true
  }
  return safeStableStringify(a) === safeStableStringify(b)
}

function computeShallowDiff(before: unknown, after: unknown): DiffEntry[] {
  if (isDeepEqual(before, after)) {
    return []
  }

  if (!isPlainObject(before) || !isPlainObject(after)) {
    return [{ path: '(root)', kind: 'changed', before, after }]
  }

  const keys = new Set<string>([...Object.keys(before), ...Object.keys(after)])
  const sorted = Array.from(keys).sort((a, b) => a.localeCompare(b))

  const entries: DiffEntry[] = []
  for (const key of sorted) {
    const hasBefore = Object.prototype.hasOwnProperty.call(before, key)
    const hasAfter = Object.prototype.hasOwnProperty.call(after, key)
    if (!hasBefore && hasAfter) {
      entries.push({ path: key, kind: 'added', after: after[key] })
      continue
    }
    if (hasBefore && !hasAfter) {
      entries.push({ path: key, kind: 'removed', before: before[key] })
      continue
    }

    const left = before[key]
    const right = after[key]
    if (!isDeepEqual(left, right)) {
      entries.push({ path: key, kind: 'changed', before: left, after: right })
    }
  }

  return entries
}

function resolveKindColor(kind: DiffKind): BadgeProps['color'] {
  switch (kind) {
    case 'added':
      return 'success'
    case 'removed':
      return 'error'
    case 'changed':
      return 'info'
    default:
      return 'default'
  }
}

function resolveKindLabel(kind: DiffKind) {
  switch (kind) {
    case 'added':
      return 'add'
    case 'removed':
      return 'remove'
    case 'changed':
      return 'change'
    default:
      return kind
  }
}

export function JsonDiffView({ before, after, maxEntries = 60, className }: JsonDiffViewProps) {
  const rawEntries = useMemo(() => computeShallowDiff(before, after), [before, after])
  const entries = rawEntries.slice(0, Math.max(0, maxEntries))
  const truncated = rawEntries.length > entries.length

  const counts = useMemo(() => {
    const result: Record<DiffKind, number> = { added: 0, removed: 0, changed: 0 }
    for (const entry of entries) {
      result[entry.kind] += 1
    }
    return result
  }, [entries])

  if (rawEntries.length === 0) {
    return <div className={cn('text-[11px] text-muted-foreground', className)}>No changes.</div>
  }

  return (
    <div className={cn('space-y-2', className)}>
      <div className="flex flex-wrap items-center gap-2">
        <Badge color="info" variant="soft" size="sm">
          {`diff ${rawEntries.length}`}
        </Badge>
        {counts.added > 0 ? (
          <Badge color="success" variant="outline" size="sm">
            {`add ${counts.added}`}
          </Badge>
        ) : null}
        {counts.changed > 0 ? (
          <Badge color="info" variant="outline" size="sm">
            {`change ${counts.changed}`}
          </Badge>
        ) : null}
        {counts.removed > 0 ? (
          <Badge color="error" variant="outline" size="sm">
            {`remove ${counts.removed}`}
          </Badge>
        ) : null}
        {truncated ? (
          <span className="text-[11px] text-muted-foreground">{`showing ${entries.length}/${rawEntries.length}`}</span>
        ) : null}
      </div>

      <div className="space-y-2">
        {entries.map((entry, idx) => {
          const beforeText = entry.before === undefined ? null : safeStableStringify(entry.before)
          const afterText = entry.after === undefined ? null : safeStableStringify(entry.after)

          return (
            <div
              key={`${entry.kind}:${entry.path}:${idx}`}
              className="rounded-md border border-border/60 bg-muted/10 px-2.5 py-2 text-xs"
            >
              <div className="flex flex-wrap items-center gap-2">
                <Badge color={resolveKindColor(entry.kind)} variant="soft" size="sm">
                  {resolveKindLabel(entry.kind)}
                </Badge>
                <CopyPill value={entry.path} title="Copy path" />
              </div>

              <div className="mt-2 grid grid-cols-1 gap-2 sm:grid-cols-2">
                <div>
                  <div className="text-[11px] font-medium text-secondary-foreground">Before</div>
                  {beforeText ? (
                    <ShowMore maxLines={8} className="mt-1">
                      <pre className="whitespace-pre-wrap break-words font-mono text-[11px] text-muted-foreground">
                        {beforeText}
                      </pre>
                    </ShowMore>
                  ) : (
                    <div className="mt-1 text-[11px] text-muted-foreground">(none)</div>
                  )}
                </div>

                <div>
                  <div className="text-[11px] font-medium text-secondary-foreground">After</div>
                  {afterText ? (
                    <ShowMore maxLines={8} className="mt-1">
                      <pre className="whitespace-pre-wrap break-words font-mono text-[11px] text-muted-foreground">
                        {afterText}
                      </pre>
                    </ShowMore>
                  ) : (
                    <div className="mt-1 text-[11px] text-muted-foreground">(none)</div>
                  )}
                </div>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
