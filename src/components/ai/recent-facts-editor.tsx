import { useCallback, useMemo, useState } from 'react'

import { Button, Input, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Textarea } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

export type RecentFactDraft = {
  summary: string
  source_ref?: string
  confidence?: 'accepted' | 'proposed' | string
}

export type RecentFactsDraft = {
  facts: RecentFactDraft[]
}

export type RecentFactsEditorInitial = {
  facts?: RecentFactDraft[]
}

export type RecentFactsEditorProps = {
  initial?: RecentFactsEditorInitial | null
  onSave?: (draft: RecentFactsDraft) => void | Promise<void>
  onCancel?: () => void
  onSaved?: () => void
  disabled?: boolean
  className?: string
}

function normalizeItem(value: string) {
  return value.trim().replaceAll(/\s+/g, ' ')
}

function normalizeConfidence(value: string): 'accepted' | 'proposed' {
  return value === 'accepted' ? 'accepted' : 'proposed'
}

export function RecentFactsEditor({
  initial,
  onSave,
  onCancel,
  onSaved,
  disabled,
  className,
}: RecentFactsEditorProps) {
  const [facts, setFacts] = useState<RecentFactDraft[]>(() => Array.isArray(initial?.facts) ? initial.facts : [])
  const [newSummary, setNewSummary] = useState('')
  const [newSourceRef, setNewSourceRef] = useState('')
  const [newConfidence, setNewConfidence] = useState<'accepted' | 'proposed'>('proposed')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const canSave = useMemo(() => {
    if (disabled) return false
    if (!onSave) return false
    if (saving) return false
    return true
  }, [disabled, onSave, saving])

  const canAdd = useMemo(() => Boolean(normalizeItem(newSummary)), [newSummary])

  const addFact = useCallback(() => {
    const summary = normalizeItem(newSummary)
    if (!summary) return

    const sourceRef = normalizeItem(newSourceRef)
    const confidence = normalizeConfidence(newConfidence)

    setFacts((prev) => ([
      ...prev,
      {
        summary,
        source_ref: sourceRef || undefined,
        confidence,
      },
    ]))
    setNewSummary('')
    setNewSourceRef('')
    setNewConfidence('proposed')
  }, [newConfidence, newSourceRef, newSummary])

  const removeFact = useCallback((idx: number) => {
    setFacts((prev) => prev.filter((_, i) => i !== idx))
  }, [])

  const handleSave = useCallback(async () => {
    if (!canSave || !onSave) return

    setError(null)
    setSaving(true)
    try {
      const normalizedFacts = facts
        .map((f) => ({
          summary: normalizeItem(f.summary),
          source_ref: normalizeItem(f.source_ref ?? '') || undefined,
          confidence: normalizeConfidence(String(f.confidence ?? 'proposed')),
        }))
        .filter((f) => Boolean(f.summary))

      await onSave({ facts: normalizedFacts })
      onSaved?.()
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }, [canSave, facts, onSave, onSaved])

  return (
    <div className={cn('rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs', className)}>
      <div className="flex items-center justify-between gap-2">
        <div className="font-medium text-secondary-foreground">Edit recent facts</div>
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
          <Button size="sm" className="text-xs" onClick={handleSave} disabled={!canSave}>
            {saving ? 'Saving…' : 'Save'}
          </Button>
        </div>
      </div>

      {error ? (
        <div className="mt-2 text-xs text-destructive bg-danger-10 rounded px-2 py-1">
          {error}
        </div>
      ) : null}

      <div className="mt-2 space-y-2">
        {facts.length > 0 ? (
          <ul className="space-y-1">
            {facts.map((f, idx) => (
              <li key={`${f.summary}-${idx}`} className="flex items-start justify-between gap-2 rounded border border-border/60 bg-background px-2 py-1">
                <div className="min-w-0">
                  <div className="break-words text-muted-foreground">{f.summary}</div>
                  <div className="mt-0.5 flex flex-wrap gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
                    <span className="opacity-70">{`[${normalizeConfidence(String(f.confidence ?? 'proposed'))}]`}</span>
                    {f.source_ref ? (
                      <span className="font-mono opacity-70" title={f.source_ref}>{f.source_ref}</span>
                    ) : null}
                  </div>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  className="text-xs shrink-0"
                  onClick={() => removeFact(idx)}
                  disabled={disabled || saving}
                >
                  Remove
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <div className="text-muted-foreground">No facts yet. (You can save an empty list to create the artifact.)</div>
        )}

        <div className="rounded-md border border-border/60 bg-background px-2 py-2">
          <div className="text-[11px] font-medium text-secondary-foreground">Add fact</div>
          <Textarea
            value={newSummary}
            onChange={(e) => setNewSummary(e.target.value)}
            placeholder="Short summary of the fact…"
            className="mt-1 min-h-[56px]"
            autoResize
            disabled={disabled || saving}
          />
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <Input
              value={newSourceRef}
              onChange={(e) => setNewSourceRef(e.target.value)}
              placeholder="source_ref (optional)"
              className="flex-1 min-w-[180px]"
              disabled={disabled || saving}
            />
            <Select value={newConfidence} onValueChange={(v) => setNewConfidence(normalizeConfidence(v))}>
              <SelectTrigger size="xs" width="auto" className="min-w-[132px]" disabled={disabled || saving}>
                <SelectValue placeholder="confidence" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="proposed">proposed</SelectItem>
                <SelectItem value="accepted">accepted</SelectItem>
              </SelectContent>
            </Select>
            <Button
              size="sm"
              variant="outline"
              className="text-xs"
              onClick={addFact}
              disabled={disabled || saving || !canAdd}
            >
              Add
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}

export default RecentFactsEditor
