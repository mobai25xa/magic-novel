import { Clock3 } from 'lucide-react'

import type { AgentTodoState } from '@/agent/types'

import type { AgentChatPanelViewProps } from '../panel/view/agent-chat-panel-view-types'
import { handleToggleRun } from '../panel/view/agent-chat-panel-view-toggle-run'
import { useAiTranslations } from '../ai-hooks'
import { ChatInput } from '../input/ChatInput'
import { ActionBar } from '../input/ActionBar'
import { TodoStatusBar } from './todo-status-bar'

type AgentChatPanelInputShellProps = Pick<
  AgentChatPanelViewProps,
  | 'running'
  | 'input'
  | 'inputDisabled'
  | 'inputPlaceholder'
  | 'sessionCanContinue'
  | 'onInputChange'
  | 'onSend'
  | 'onCancel'
  | 'models'
  | 'selectedModel'
  | 'onSelectModel'
  | 'approvalMode'
  | 'elapsedTime'
  | 'showTimer'
> & {
  todoState: AgentTodoState
}

export function AgentChatPanelInputShell(input: AgentChatPanelInputShellProps) {
  const ai = useAiTranslations()

  return (
    <div className="editor-shell-ai-input-wrap">
      {input.showTimer && input.elapsedTime ? (
        <div className="flex items-center gap-1.5 px-3 py-1.5 mb-1.5 text-xs text-muted-foreground ai-animate-pulse chat-input-runtime-hint">
          <Clock3 className="h-3 w-3" />
          <span>{ai.panel.generating} · {input.elapsedTime}</span>
        </div>
      ) : null}

      <TodoStatusBar todoState={input.todoState} />

      <div className="chat-input-shell" data-disabled={input.inputDisabled ? 'true' : 'false'}>
        <ChatInput
          value={input.input}
          onChange={input.onInputChange}
          onSend={() => { void input.onSend() }}
          disabled={input.inputDisabled}
          placeholder={input.inputPlaceholder || ai.panel.inputPlaceholder}
        />

        <ActionBar
          models={input.models}
          selectedModel={input.selectedModel}
          onSelectModel={input.onSelectModel}
          approvalMode={input.approvalMode}
          running={input.running}
          inputEmpty={!input.input.trim()}
          canContinue={input.sessionCanContinue}
          onToggleRun={() => handleToggleRun(input)}
        />
      </div>
    </div>
  )
}
