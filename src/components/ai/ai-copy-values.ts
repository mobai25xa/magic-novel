import type { AgentUiTurnPhase, AgentUiToolStep } from '@/lib/agent-chat/types'

import type { aiEn } from '@/i18n/locales/ai'

type AiCopy = typeof aiEn

export function buildTurnPhaseLabel(copy: AiCopy, phase: AgentUiTurnPhase) {
  return copy.turn.phaseLabel[phase]
}

export function buildTurnPhaseCopy(copy: AiCopy, phase: AgentUiTurnPhase) {
  return copy.turn.phaseCopy[phase]
}

export function buildToolStatusLabel(copy: AiCopy, step: AgentUiToolStep) {
  if (step.status === 'waiting_confirmation' && step.progress === 'waiting_askuser') {
    return copy.tool.statusLabel.waiting_askuser
  }
  return copy.tool.statusLabel[step.status]
}

export function buildToolStatusCopy(input: {
  copy: AiCopy
  toolName: string
  status: AgentUiToolStep['status']
  progress?: string
  message?: string
}) {
  const suffix = (() => {
    if (input.status === 'running') return input.copy.tool.statusCopy.running
    if (input.status === 'waiting_confirmation') {
      return input.progress === 'waiting_askuser'
        ? input.copy.tool.statusCopy.waiting_askuser
        : input.copy.tool.statusCopy.waiting_confirmation
    }
    if (input.status === 'success') return input.copy.tool.statusCopy.success
    if (input.status === 'cancelled') return input.copy.tool.statusCopy.cancelled
    return `${input.copy.tool.statusCopy.failed}: ${input.message || input.copy.tool.statusCopy.retryHint}`
  })()

  return `${input.toolName} ${suffix}`
}

export function buildJumpToLatestLabel(copy: AiCopy, unseenCount: number) {
  return unseenCount > 0
    ? `${copy.panel.jumpToLatest} (+${unseenCount})`
    : copy.panel.jumpToLatest
}

export function buildToolDiffSummaryLabel(copy: AiCopy, plus: number, minus: number) {
  return `${copy.tool.diffPrefix}: +${plus} / -${minus}`
}
