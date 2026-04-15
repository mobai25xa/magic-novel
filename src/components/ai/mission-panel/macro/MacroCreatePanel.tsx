import { useCallback, useMemo, useState } from 'react'

import { missionMacroCreateFeature } from '@/features/agent-chat'

import type { MissionStatusPayload } from '../types'
import { MacroCreateFormView, type MacroCreateFormState } from './MacroCreateFormView'
import { parseMacroChapterTargetsText } from './targets'

const DEFAULT_TARGETS_TEXT = [
  '[',
  '  { "chapter_ref": "vol_1/ch_001", "write_path": "vol_1/ch_001.json", "display_title": "Chapter 1" },',
  '  { "chapter_ref": "vol_1/ch_002", "write_path": "vol_1/ch_002.json", "display_title": "Chapter 2" },',
  '  { "chapter_ref": "vol_1/ch_003", "write_path": "vol_1/ch_003.json", "display_title": "Chapter 3" }',
  ']',
].join('\n')

type MacroCreatePanelProps = {
  projectPath: string
  statusDetail: MissionStatusPayload | null
  loading: boolean
  onRefresh: () => void
  onOpenDetails: () => void
}

function useMacroCreateForm(input: { statusDetail: MissionStatusPayload | null }) {
  const [createOpen, setCreateOpen] = useState(true)
  const [creating, setCreating] = useState(false)
  const [createError, setCreateError] = useState<string | null>(null)
  const [form, setForm] = useState<MacroCreateFormState>({
    objective: '',
    workflowKind: 'book',
    tokenBudget: 'medium',
    strictReview: false,
    autoFixOnBlock: true,
    targetsText: DEFAULT_TARGETS_TEXT,
  })
  const suggestedObjective = useMemo(() => {
    const title = input.statusDetail?.features?.title
    return typeof title === 'string' ? title.trim() : ''
  }, [input.statusDetail?.features?.title])

  return { createOpen, setCreateOpen, creating, setCreating, createError, setCreateError, form, setForm, suggestedObjective }
}

export function MacroCreatePanel({
  projectPath,
  statusDetail,
  loading,
  onRefresh,
  onOpenDetails,
}: MacroCreatePanelProps) {
  const {
    createOpen,
    setCreateOpen,
    creating,
    setCreating,
    createError,
    setCreateError,
    form,
    setForm,
    suggestedObjective,
  } = useMacroCreateForm({ statusDetail })

  const handleCreate = useCallback(async () => {
    setCreateError(null)
    setCreating(true)
    try {
      const objective = form.objective.trim() || suggestedObjective
      if (!objective) {
        setCreateError('Objective is required')
        return
      }

      const parsed = parseMacroChapterTargetsText(form.targetsText)
      if (parsed.error) {
        setCreateError(parsed.error)
        return
      }

      await missionMacroCreateFeature({
        project_path: projectPath,
        objective,
        workflow_kind: form.workflowKind,
        chapter_targets: parsed.targets,
        strict_review: form.strictReview,
        auto_fix_on_block: form.autoFixOnBlock,
        token_budget: form.tokenBudget,
      })

      onRefresh()
      onOpenDetails()
    } catch (e) {
      setCreateError(String(e))
    } finally {
      setCreating(false)
    }
  }, [
    form.autoFixOnBlock,
    form.objective,
    form.strictReview,
    form.targetsText,
    form.tokenBudget,
    form.workflowKind,
    onOpenDetails,
    onRefresh,
    projectPath,
    setCreateError,
    setCreating,
    suggestedObjective,
  ])

  return (
    <MacroCreateFormView
      open={createOpen}
      onOpenChange={setCreateOpen}
      creating={creating}
      loading={loading}
      error={createError}
      form={form}
      setForm={setForm}
      onCreate={handleCreate}
    />
  )
}
