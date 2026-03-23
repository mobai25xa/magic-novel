import { useCallback, useMemo, useState } from 'react'

import { Button, Input, Textarea } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

export type ActiveCastEntryDraft = {
  character_ref: string
  current_state_summary: string
  must_keep_voice_signals?: string[]
}

export type ActiveCastDraft = {
  cast: ActiveCastEntryDraft[]
}

export type ActiveCastEditorInitial = {
  cast?: ActiveCastEntryDraft[]
}

export type ActiveCastEditorProps = {
  initial?: ActiveCastEditorInitial | null
  onSave?: (draft: ActiveCastDraft) => void | Promise<void>
  onCancel?: () => void
  onSaved?: () => void
  disabled?: boolean
  className?: string
}

function normalizeItem(value: string) {
  return value.trim().replaceAll(/\s+/g, ' ')
}

function normalizeSignals(value: string): string[] | undefined {
  const list = value
    .split(',')
    .map((s) => normalizeItem(s))
    .filter(Boolean)

  return list.length > 0 ? list : undefined
}

export function ActiveCastEditor({
  initial,
  onSave,
  onCancel,
  onSaved,
  disabled,
  className,
}: ActiveCastEditorProps) {
  const [cast, setCast] = useState<ActiveCastEntryDraft[]>(() => Array.isArray(initial?.cast) ? initial.cast : [])
  const [newCharacterRef, setNewCharacterRef] = useState('')
  const [newStateSummary, setNewStateSummary] = useState('')
  const [newVoiceSignals, setNewVoiceSignals] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const canSave = useMemo(() => {
    if (disabled) return false
    if (!onSave) return false
    if (saving) return false
    return true
  }, [disabled, onSave, saving])

  const canAdd = useMemo(() => {
    if (!normalizeItem(newCharacterRef)) return false
    if (!normalizeItem(newStateSummary)) return false
    return true
  }, [newCharacterRef, newStateSummary])

  const addEntry = useCallback(() => {
    const characterRef = normalizeItem(newCharacterRef)
    const stateSummary = normalizeItem(newStateSummary)
    if (!characterRef || !stateSummary) return

    const voiceSignals = normalizeSignals(newVoiceSignals)
    setCast((prev) => ([
      ...prev,
      {
        character_ref: characterRef,
        current_state_summary: stateSummary,
        must_keep_voice_signals: voiceSignals,
      },
    ]))
    setNewCharacterRef('')
    setNewStateSummary('')
    setNewVoiceSignals('')
  }, [newCharacterRef, newStateSummary, newVoiceSignals])

  const removeEntry = useCallback((idx: number) => {
    setCast((prev) => prev.filter((_, i) => i !== idx))
  }, [])

  const handleSave = useCallback(async () => {
    if (!canSave || !onSave) return

    setError(null)
    setSaving(true)
    try {
      const normalizedCast = cast
        .map((c) => ({
          character_ref: normalizeItem(c.character_ref),
          current_state_summary: normalizeItem(c.current_state_summary),
          must_keep_voice_signals: Array.isArray(c.must_keep_voice_signals) && c.must_keep_voice_signals.length > 0
            ? c.must_keep_voice_signals.map((s) => normalizeItem(s)).filter(Boolean)
            : undefined,
        }))
        .filter((c) => Boolean(c.character_ref) && Boolean(c.current_state_summary))

      await onSave({ cast: normalizedCast })
      onSaved?.()
    } catch (e) {
      setError(String(e))
    } finally {
      setSaving(false)
    }
  }, [canSave, cast, onSave, onSaved])

  return (
    <div className={cn('rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs', className)}>
      <div className="flex items-center justify-between gap-2">
        <div className="font-medium text-secondary-foreground">Edit active cast</div>
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
        {cast.length > 0 ? (
          <ul className="space-y-1">
            {cast.map((c, idx) => (
              <li key={`${c.character_ref}-${idx}`} className="flex items-start justify-between gap-2 rounded border border-border/60 bg-background px-2 py-1">
                <div className="min-w-0">
                  <div className="flex items-baseline gap-2">
                    <span className="font-mono text-[11px] text-foreground/80" title={c.character_ref}>
                      {c.character_ref}
                    </span>
                    <span className="break-words text-muted-foreground">{c.current_state_summary}</span>
                  </div>
                  {c.must_keep_voice_signals && c.must_keep_voice_signals.length > 0 ? (
                    <div className="mt-0.5 text-[11px] text-muted-foreground opacity-70">
                      {`voice: ${c.must_keep_voice_signals.join(', ')}`}
                    </div>
                  ) : null}
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  className="text-xs shrink-0"
                  onClick={() => removeEntry(idx)}
                  disabled={disabled || saving}
                >
                  Remove
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <div className="text-muted-foreground">No cast notes yet. (You can save an empty list to create the artifact.)</div>
        )}

        <div className="rounded-md border border-border/60 bg-background px-2 py-2">
          <div className="text-[11px] font-medium text-secondary-foreground">Add cast entry</div>
          <div className="mt-1 grid grid-cols-1 gap-2">
            <Input
              value={newCharacterRef}
              onChange={(e) => setNewCharacterRef(e.target.value)}
              placeholder="character_ref (e.g. char:alice)"
              disabled={disabled || saving}
            />
            <Textarea
              value={newStateSummary}
              onChange={(e) => setNewStateSummary(e.target.value)}
              placeholder="current_state_summary…"
              className="min-h-[56px]"
              autoResize
              disabled={disabled || saving}
            />
            <Input
              value={newVoiceSignals}
              onChange={(e) => setNewVoiceSignals(e.target.value)}
              placeholder="must_keep_voice_signals (comma-separated, optional)"
              disabled={disabled || saving}
            />
            <div className="flex justify-end">
              <Button
                size="sm"
                variant="outline"
                className="text-xs"
                onClick={addEntry}
                disabled={disabled || saving || !canAdd}
              >
                Add
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ActiveCastEditor

