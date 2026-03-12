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
  maxDepth?: number
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

function computeDeepDiff(
  before: unknown,
  after: unknown,
  options: { maxDepth: number; limit: number },
): { entries: DiffEntry[]; truncated: boolean } {
  if (isDeepEqual(before, after)) {
    return { entries: [], truncated: false }
  }

  const entries: DiffEntry[] = []
  let truncated = false

  const beforeStack = new WeakSet<object>()
  const afterStack = new WeakSet<object>()

  const pushEntry = (entry: DiffEntry) => {
    if (truncated) {
      return
    }
    entries.push(entry)
    if (entries.length >= options.limit) {
      truncated = true
    }
  }

  const joinObjectPath = (parent: string, key: string) => (parent === '(root)' ? key : `${parent}.${key}`)
  const joinArrayPath = (parent: string, index: number) => (parent === '(root)' ? `[${index}]` : `${parent}[${index}]`)

  const walk = (left: unknown, right: unknown, path: string, depth: number) => {
    if (truncated) {
      return
    }

    if (Object.is(left, right)) {
      return
    }

    if (depth >= options.maxDepth) {
      if (!isDeepEqual(left, right)) {
        pushEntry({ path, kind: 'changed', before: left, after: right })
      }
      return
    }

    const isLeftObj = Boolean(left) && typeof left === 'object'
    const isRightObj = Boolean(right) && typeof right === 'object'
    const leftObj = isLeftObj ? (left as object) : null
    const rightObj = isRightObj ? (right as object) : null

    if (leftObj && beforeStack.has(leftObj)) {
      if (!isDeepEqual(left, right)) {
        pushEntry({ path, kind: 'changed', before: left, after: right })
      }
      return
    }

    if (rightObj && afterStack.has(rightObj)) {
      if (!isDeepEqual(left, right)) {
        pushEntry({ path, kind: 'changed', before: left, after: right })
      }
      return
    }

    if (leftObj) {
      beforeStack.add(leftObj)
    }
    if (rightObj) {
      afterStack.add(rightObj)
    }

    try {
      if (Array.isArray(left) && Array.isArray(right)) {
        const maxLen = Math.max(left.length, right.length)
        for (let i = 0; i < maxLen; i += 1) {
          if (truncated) {
            return
          }

          const hasLeft = i < left.length
          const hasRight = i < right.length
          const nextPath = joinArrayPath(path, i)

          if (!hasLeft && hasRight) {
            pushEntry({ path: nextPath, kind: 'added', after: right[i] })
            continue
          }
          if (hasLeft && !hasRight) {
            pushEntry({ path: nextPath, kind: 'removed', before: left[i] })
            continue
          }
          walk(left[i], right[i], nextPath, depth + 1)
        }
        return
      }

      if (isPlainObject(left) && isPlainObject(right)) {
        const keys = new Set<string>([...Object.keys(left), ...Object.keys(right)])
        const sorted = Array.from(keys).sort((a, b) => a.localeCompare(b))

        for (const key of sorted) {
          if (truncated) {
            return
          }

          const hasLeft = Object.prototype.hasOwnProperty.call(left, key)
          const hasRight = Object.prototype.hasOwnProperty.call(right, key)
          const nextPath = joinObjectPath(path, key)

          if (!hasLeft && hasRight) {
            pushEntry({ path: nextPath, kind: 'added', after: right[key] })
            continue
          }
          if (hasLeft && !hasRight) {
            pushEntry({ path: nextPath, kind: 'removed', before: left[key] })
            continue
          }

          walk(left[key], right[key], nextPath, depth + 1)
        }
        return
      }

      if (!isDeepEqual(left, right)) {
        pushEntry({ path, kind: 'changed', before: left, after: right })
      }
    } finally {
      if (leftObj) {
        beforeStack.delete(leftObj)
      }
      if (rightObj) {
        afterStack.delete(rightObj)
      }
    }
  }

  walk(before, after, '(root)', 0)

  return { entries, truncated }
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

export function JsonDiffView({ before, after, maxEntries = 60, maxDepth = 4, className }: JsonDiffViewProps) {
  const computeLimit = Math.max(0, maxEntries) * 6 + 40
  const effectiveMaxDepth = Math.max(0, Math.floor(maxDepth))

  const raw = useMemo(
    () => computeDeepDiff(before, after, { maxDepth: effectiveMaxDepth, limit: computeLimit }),
    [before, after, computeLimit, effectiveMaxDepth],
  )

  const entries = raw.entries.slice(0, Math.max(0, maxEntries))
  const truncated = raw.truncated || raw.entries.length > entries.length

  const counts = useMemo(() => {
    const result: Record<DiffKind, number> = { added: 0, removed: 0, changed: 0 }
    for (const entry of entries) {
      result[entry.kind] += 1
    }
    return result
  }, [entries])

  if (raw.entries.length === 0) {
    return <div className={cn('text-[11px] text-muted-foreground', className)}>No changes.</div>
  }

  return (
    <div className={cn('space-y-2', className)}>
      <div className="flex flex-wrap items-center gap-2">
        <Badge color="info" variant="soft" size="sm">
          {`diff ${raw.entries.length}${raw.truncated ? '+' : ''}`}
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
          <span className="text-[11px] text-muted-foreground">{`showing ${entries.length}/${raw.entries.length}${raw.truncated ? '+' : ''}`}</span>
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
