import type { AgentUiToolStep } from '@/lib/agent-chat/types'

export function summarizeToolStep(step: AgentUiToolStep) {
  const duration = typeof step.durationMs === 'number' ? `${step.durationMs}ms` : '--'
  const code = step.errorCode ? ` · ${step.errorCode}` : ''

  return {
    duration,
    code,
  }
}
