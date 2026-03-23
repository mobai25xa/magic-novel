import { useMemo } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'
import { AiToolContent } from '@/magic-ui/components'

import { KnowledgeApplyStatusCard } from '../../knowledge/knowledge-apply-status-card'
import { CopyPill } from '../../knowledge/copy-pill'
import { parseToolOutput, resolveStepOpenRef } from '../tool-view-utils'
import { GenericToolView } from './GenericToolView'
import { openEditorTarget, parseEditorTargetRef } from '@/features/editor-navigation/open-editor-target'

type KnowledgeWriteDeltaLike = {
  knowledge_delta_id: string
  status: string
  next_action?: string
  scope_ref?: string
  target_ref?: string
  path?: string
  accepted_item_ids?: string[]
  rejected_item_ids?: string[]
  conflicts?: Array<{ type: string; message: string } & Record<string, unknown>>
  rollback?: { kind?: string; token?: string }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as Record<string, unknown>
}

function asString(value: unknown): string | undefined {
  const text = typeof value === 'string' ? value.trim() : ''
  return text ? text : undefined
}

function asStringArray(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) {
    return undefined
  }
  const result = value.filter((item): item is string => typeof item === 'string' && item.trim().length > 0)
  return result.length > 0 ? result : undefined
}

function coerceConflicts(value: unknown): KnowledgeWriteDeltaLike['conflicts'] {
  if (!Array.isArray(value)) {
    return undefined
  }

  const conflicts = value
    .map((row) => asRecord(row))
    .filter((row): row is Record<string, unknown> => Boolean(row))
    .map((row) => ({
      ...row,
      type: asString(row.type) ?? 'conflict',
      message: asString(row.message) ?? '',
    }))

  return conflicts.length > 0 ? conflicts : undefined
}

function coerceKnowledgeWriteDelta(value: unknown): KnowledgeWriteDeltaLike | null {
  const direct = asRecord(value)
  if (!direct) {
    return null
  }

  const knowledgeDeltaId = asString(direct.knowledge_delta_id) ?? asString(direct.delta_id)
  const status = asString(direct.status)
  if (knowledgeDeltaId && status) {
    const rollback = asRecord(direct.rollback)
    const rollbackToken = rollback ? asString(rollback.token) : undefined
    return {
      knowledge_delta_id: knowledgeDeltaId,
      status,
      next_action: asString(direct.next_action),
      scope_ref: asString(direct.scope_ref),
      target_ref: asString(direct.target_ref),
      path: asString(direct.path),
      accepted_item_ids: asStringArray(direct.accepted_item_ids),
      rejected_item_ids: asStringArray(direct.rejected_item_ids),
      conflicts: coerceConflicts(direct.conflicts),
      rollback: rollbackToken ? { token: rollbackToken } : undefined,
    }
  }

  const data = asRecord(direct.data)
  if (data) {
    const nested = coerceKnowledgeWriteDelta(data)
    if (nested) {
      return nested
    }
  }

  const result = asRecord(direct.result)
  const preview = asRecord(result?.preview)
  if (preview) {
    const nested = coerceKnowledgeWriteDelta(preview)
    if (nested) {
      return nested
    }
  }

  return null
}

type KnowledgeToolViewProps = {
  step: AgentUiToolStep
}

export function KnowledgeToolView({ step }: KnowledgeToolViewProps) {
  const parsed = useMemo(() => parseToolOutput(step.rawOutput), [step.rawOutput])
  const delta = useMemo(
    () => coerceKnowledgeWriteDelta(parsed ?? step.outputPreview),
    [parsed, step.outputPreview],
  )

  if (!delta) {
    return <GenericToolView step={step} />
  }

  const pills = [
    delta.scope_ref ? { value: delta.scope_ref, title: 'Copy scope_ref' } : null,
    delta.target_ref ? { value: delta.target_ref, title: 'Copy target_ref' } : null,
    delta.path ? { value: delta.path, title: 'Copy path' } : null,
  ].filter(Boolean) as Array<{ value: string; title: string }>

  const openCandidate = useMemo(() => {
    const fromDelta = delta.target_ref || delta.path
    if (typeof fromDelta === 'string' && fromDelta.trim() && parseEditorTargetRef(fromDelta)) {
      return fromDelta.trim()
    }

    const fromStep = resolveStepOpenRef(step)
    if (fromStep && parseEditorTargetRef(fromStep)) {
      return fromStep
    }

    return ''
  }, [delta.path, delta.target_ref, step])

  const canOpen = Boolean(openCandidate)

  return (
    <AiToolContent className="space-y-2">
      <KnowledgeApplyStatusCard
        delta={delta}
        showActions={false}
      />

      {delta.next_action ? (
        <div className="text-[11px] text-muted-foreground">
          {`next_action: ${delta.next_action}`}
        </div>
      ) : null}

      {pills.length > 0 || canOpen ? (
        <div className="flex flex-wrap items-center gap-1.5">
          {pills.map((pill) => (
            <CopyPill key={pill.value} value={pill.value} title={pill.title} />
          ))}
          {canOpen ? (
            <button
              type="button"
              onClick={() => { void openEditorTarget(openCandidate, { revealLeftTree: true, switchLeftTab: true }) }}
              className="inline-flex items-center gap-1.5 rounded border border-border/60 bg-background px-2 py-1 text-[11px] text-muted-foreground hover:text-foreground hover:bg-muted/30 transition-colors"
            >
              打开
            </button>
          ) : null}
        </div>
      ) : null}
    </AiToolContent>
  )
}
