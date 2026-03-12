import { useCallback, useMemo, useState } from 'react'

import { Button, Input, Tag, Textarea } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

export type ChapterCardDraft = {
  objective: string
  hard_constraints: string[]
  success_criteria: string[]
}

export type ChapterCardEditorInitial = {
  objective?: string
  hard_constraints?: string[]
  success_criteria?: string[]
}

export type ChapterCardEditorProps = {
  initial?: ChapterCardEditorInitial | null
  onSave?: (draft: ChapterCardDraft) => void | Promise<void>
  onCancel?: () => void
  onSaved?: () => void
  disabled?: boolean
  className?: string
}

function normalizeItem(value: string) {
  return value.trim().replaceAll(/\s+/g, ' ')
}

function normalizeList(values: string[]) {
  const out: string[] = []
  const seen = new Set<string>()

  for (const raw of values) {
    const v = normalizeItem(raw)
    if (!v) continue

    const key = v.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    out.push(v)
  }

  return out
}

export function ChapterCardEditor({
  initial,
  onSave,
  onCancel,
  onSaved,
  disabled,
  className,
}: ChapterCardEditorProps) {
  const [objective, setObjective] = useState(() => String(initial?.objective ?? ''))
  const [hardConstraints, setHardConstraints] = useState<string[]>(() => normalizeList(initial?.hard_constraints ?? []))
  const [successCriteria, setSuccessCriteria] = useState<string[]>(() => normalizeList(initial?.success_criteria ?? []))
  const [newConstraint, setNewConstraint] = useState('')
  const [newSuccess, setNewSuccess] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const canSave = useMemo(() => {
    if (disabled) return false
    if (!onSave) return false
    if (saving) return false
    return Boolean(normalizeItem(objective))
  }, [disabled, onSave, saving, objective])

  const addConstraint = useCallback(() => {
    const next = normalizeItem(newConstraint)
    if (!next) return

    setHardConstraints((prev) => normalizeList([...prev, next]))
    setNewConstraint('')
  }, [newConstraint])

  const addSuccess = useCallback(() => {
    const next = normalizeItem(newSuccess)
    if (!next) return

    setSuccessCriteria((prev) => normalizeList([...prev, next]))
    setNewSuccess('')
  }, [newSuccess])

  const handleSave = useCallback(async () => {
    if (!canSave || !onSave) return

    setError(null)
    setSaving(true)
    try {
      const draft: ChapterCardDraft = {
        objective: normalizeItem(objective),
        hard_constraints: normalizeList(hardConstraints),
        success_criteria: normalizeList(successCriteria),
      }

      await onSave(draft)
      onSaved?.()
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }, [canSave, onSave, objective, hardConstraints, successCriteria, onSaved])

  return (
    <div className={cn('rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs', className)}>
      <div className="flex items-center justify-between gap-2">
        <div className="font-medium text-secondary-foreground">Edit chapter card</div>
        <div className="flex items-center gap-2">
          {onCancel ? (
            <Button
              size="sm"
              variant="outline"
              className="text-xs"
              onClick={onCancel}
              disabled={saving}
            >
              Cancel
            </Button>
          ) : null}
          <Button
            size="sm"
            className="text-xs"
            onClick={handleSave}
            disabled={!canSave}
            title={!onSave ? 'Save handler not wired yet' : undefined}
          >
            {saving ? 'Saving…' : 'Save'}
          </Button>
        </div>
      </div>

      {error ? (
        <div className="mt-2 text-xs text-destructive bg-danger-10 rounded px-2 py-1">
          {error}
        </div>
      ) : null}

      <div className="mt-2 space-y-3">
        <div>
          <div className="text-[11px] font-medium text-secondary-foreground">Objective</div>
          <Textarea
            value={objective}
            onChange={(e) => setObjective(e.target.value)}
            placeholder="What should this chapter/task accomplish?"
            className="mt-1 min-h-[72px]"
            autoResize
            disabled={disabled || saving}
          />
        </div>

        <div>
          <div className="text-[11px] font-medium text-secondary-foreground">Constraints</div>
          {hardConstraints.length > 0 ? (
            <div className="mt-1 flex flex-wrap gap-2">
              {hardConstraints.map((c, idx) => (
                <Tag
                  key={`${c}-${idx}`}
                  variant="outline"
                  size="sm"
                  closable={!disabled && !saving}
                  onClose={() => {
                    if (disabled || saving) return
                    setHardConstraints((prev) => prev.filter((_, i) => i !== idx))
                  }}
                >
                  {c}
                </Tag>
              ))}
            </div>
          ) : (
            <div className="mt-1 text-muted-foreground">No constraints.</div>
          )}

          <div className="mt-2 flex items-center gap-2">
            <Input
              value={newConstraint}
              onChange={(e) => setNewConstraint(e.target.value)}
              placeholder="Add a constraint…"
              className="flex-1"
              disabled={disabled || saving}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  addConstraint()
                }
              }}
            />
            <Button
              size="sm"
              variant="outline"
              className="text-xs"
              onClick={addConstraint}
              disabled={disabled || saving || !normalizeItem(newConstraint)}
            >
              Add
            </Button>
          </div>
        </div>

        <div>
          <div className="text-[11px] font-medium text-secondary-foreground">Success criteria</div>
          {successCriteria.length > 0 ? (
            <div className="mt-1 flex flex-wrap gap-2">
              {successCriteria.map((c, idx) => (
                <Tag
                  key={`${c}-${idx}`}
                  variant="outline"
                  size="sm"
                  closable={!disabled && !saving}
                  onClose={() => {
                    if (disabled || saving) return
                    setSuccessCriteria((prev) => prev.filter((_, i) => i !== idx))
                  }}
                >
                  {c}
                </Tag>
              ))}
            </div>
          ) : (
            <div className="mt-1 text-muted-foreground">No success criteria.</div>
          )}

          <div className="mt-2 flex items-center gap-2">
            <Input
              value={newSuccess}
              onChange={(e) => setNewSuccess(e.target.value)}
              placeholder="Add a success criteria…"
              className="flex-1"
              disabled={disabled || saving}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  addSuccess()
                }
              }}
            />
            <Button
              size="sm"
              variant="outline"
              className="text-xs"
              onClick={addSuccess}
              disabled={disabled || saving || !normalizeItem(newSuccess)}
            >
              Add
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ChapterCardEditor
