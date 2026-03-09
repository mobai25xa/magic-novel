import { useProjectStore } from '@/state/project'

import type { AgentPanelError } from '../../agent-chat-panel-utils'
import { parseAgentError } from '../../agent-chat-panel-utils'

function withProjectPath(task: (projectPath: string) => Promise<void>, onError: (error: AgentPanelError) => void) {
  const projectPath = useProjectStore.getState().projectPath
  if (!projectPath) {
    onError(parseAgentError('E_AGENT_PROJECT_NOT_OPEN'))
    return Promise.resolve()
  }

  return task(projectPath)
}

export function runSessionAction(
  task: (projectPath: string) => Promise<void>,
  input: {
    onError: (value: AgentPanelError | null) => void
  },
) {
  input.onError(null)

  return withProjectPath(async (projectPath) => {
    try {
      await task(projectPath)
    } catch (error) {
      input.onError(parseAgentError(error))
    }
  }, (error) => input.onError(error))
}
