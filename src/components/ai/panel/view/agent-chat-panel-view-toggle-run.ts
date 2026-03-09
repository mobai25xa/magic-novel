import type { AgentChatPanelViewProps } from './agent-chat-panel-view-types'

export function handleToggleRun(input: Pick<AgentChatPanelViewProps, 'running' | 'sessionCanContinue' | 'onCancel' | 'onSend'>) {
  if (input.running) {
    input.onCancel()
    return
  }

  if (!input.sessionCanContinue) {
    return
  }

  void input.onSend()
}
