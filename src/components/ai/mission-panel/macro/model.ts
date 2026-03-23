import type { MacroWorkflowPhase } from '@/components/ai/macro'

import type { MacroStatePayload } from '../types'

function resolveMacroPhase(input: { hasMacro: boolean; liveState: string }): MacroWorkflowPhase {
  if (!input.hasMacro) {
    return 'not_created'
  }

  switch (input.liveState) {
    case 'completed':
      return 'completed'
    case 'paused':
      return 'paused'
    case 'running':
    case 'initializing':
      return 'running'
    case 'cancelled':
      return 'cancelled'
    case 'failed':
      return 'failed'
    default:
      return 'awaiting_input'
  }
}

export function deriveMacroSummary(input: {
  liveState: string
  macroState: MacroStatePayload | null
  reviewAutoFixAvailable: boolean
  reviewDecisionRequired: boolean
  knowledgeDecisionRequired: boolean
}) {
  const macroConfig = input.macroState?.config ?? null
  const macroProgress = input.macroState?.state ?? null

  const hasMacro = macroConfig !== null || macroProgress !== null
  const chapters = macroProgress?.chapters ?? []
  const currentIndex = macroProgress?.current_index ?? -1
  const currentStage = macroProgress?.current_stage ?? null

  const completedCount = chapters.filter((c) => c.status === 'completed').length
  const failedCount = chapters.filter((c) => c.status === 'failed' || c.status === 'blocked').length

  const isBlocked = currentStage === 'blocked' || currentStage === 'failed'
  const blockReason = macroProgress?.last_error?.message ?? null

  const canAutoFix = isBlocked && macroConfig?.auto_fix_on_block === true && input.reviewAutoFixAvailable
  const needsDecision = isBlocked && (input.reviewDecisionRequired || input.knowledgeDecisionRequired)
  const canResume = isBlocked

  const currentChapter = currentIndex >= 0 && currentIndex < chapters.length ? chapters[currentIndex] : null
  const phase = resolveMacroPhase({ hasMacro, liveState: input.liveState })

  const writeTargets = (macroConfig?.chapter_targets ?? []).map((t) => ({
    chapterRef: t.chapter_ref,
    writePath: t.write_path,
    displayTitle: t.display_title,
  }))

  return {
    macroConfig,
    macroProgress,
    hasMacro,
    chapters,
    currentIndex,
    currentStage,
    completedCount,
    failedCount,
    isBlocked,
    blockReason,
    canAutoFix,
    needsDecision,
    canResume,
    currentChapter,
    phase,
    writeTargets,
  }
}

