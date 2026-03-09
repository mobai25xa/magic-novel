import { cancelCurrentChatTurn, runChatTurn, useAgentChatStore } from '@/lib/agent-chat'
import type { Translations } from '@/i18n/locales/zh'
import type { ApprovalMode, CapabilityMode } from '@/state/settings'

import type { ChatContext } from './input/chat-context-types'
import { parseAgentError } from './agent-chat-panel-utils'
import type { AgentPanelError } from './agent-chat-panel-utils'

type SetLastError = (value: AgentPanelError | null) => void
type SetInput = (value: string) => void

type CreateChatPanelActionsInput = {
  approvalMode: ApprovalMode
  capabilityMode: CapabilityMode
  running: boolean
  canContinue: boolean
  runtimeState: 'ready' | 'running' | 'suspended_confirmation' | 'suspended_askuser' | 'completed' | 'failed' | 'cancelled' | 'degraded'
  setInput: SetInput
  setLastError: SetLastError
  contexts: ChatContext[]
  clearContexts: () => void
  labels: Translations['aiChat']
}

async function runWithErrorGuard(input: {
  setLastError: SetLastError
  task: () => Promise<unknown>
}) {
  input.setLastError(null)
  try {
    await input.task()
  } catch (error) {
    input.setLastError(parseAgentError(error))
  }
}

export function createChatPanelActions(input: CreateChatPanelActionsInput) {
  const runtimeOptions = {
    approvalMode: input.approvalMode,
    capabilityMode: input.capabilityMode,
  }

  const handleSend = async (rawInput: string) => {
    const text = rawInput.trim()
    if (!text || input.running || !input.canContinue) {
      return
    }

    if (input.runtimeState === 'suspended_confirmation' || input.runtimeState === 'suspended_askuser') {
      input.setLastError(parseAgentError('E_AGENT_SESSION_SUSPENDED'))
      return
    }

    if (input.runtimeState === 'degraded') {
      input.setLastError(parseAgentError('E_AGENT_SESSION_READONLY'))
      return
    }

    const contexts = input.contexts
    const contextSuffix = contexts.length > 0
      ? `\n\n${input.labels.referenceContext}\n${contexts.map((c) => `- ${c.type}: ${c.path}`).join('\n')}`
      : ''

    input.setInput('')
    input.clearContexts()
    await runWithErrorGuard({
      setLastError: input.setLastError,
      task: () => runChatTurn(text + contextSuffix, runtimeOptions),
    })
  }

  const handleRetryStep = async (turnId: number, callId: string) => {
    if (input.running) {
      return
    }

    await runWithErrorGuard({
      setLastError: input.setLastError,
      task: () => useAgentChatStore.getState().retryStep(turnId, callId, runtimeOptions),
    })
  }

  return {
    handleSend,
    handleRetryStep,
    handleCancel: () => {
      cancelCurrentChatTurn()
      input.setLastError(null)
    },
  }
}
