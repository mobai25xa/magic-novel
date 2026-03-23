import { Clock, Flag, Plus } from 'lucide-react'

import { useAiTranslations } from '../ai-hooks'

type AgentChatPanelHeaderProps = {
  running: boolean
  canStartNewSession?: boolean
  onStartNewSession: () => Promise<void>
  onToggleHistoryPage: () => void
  onOpenMissionPanel: () => void
  historyPageOpen: boolean
  historyEnabled: boolean
  sessionLoading: boolean
  missionDisabled: boolean
}

export function AgentChatPanelHeader(input: AgentChatPanelHeaderProps) {
  const ai = useAiTranslations()

  return (
    <div className="editor-shell-ai-chat-header">
      <div className="editor-shell-ai-chat-header-left">
        <span className="editor-shell-ai-chat-model-dot" />
        <span className="editor-shell-ai-chat-model">magic</span>
      </div>

      <div className="editor-shell-ai-chat-header-right">
        <button
          type="button"
          className="editor-shell-ai-chat-header-btn"
          onClick={input.onOpenMissionPanel}
          title={input.missionDisabled ? ai.panel.missionUnavailable : ai.panel.mission}
          aria-label={ai.panel.mission}
          disabled={input.missionDisabled}
        >
          <Flag size={13} />
        </button>
        <button
          type="button"
          className="editor-shell-ai-chat-header-btn"
          data-active={input.historyPageOpen ? 'true' : 'false'}
          onClick={input.onToggleHistoryPage}
          title={ai.panel.history}
          aria-label={ai.panel.history}
          disabled={input.sessionLoading || !input.historyEnabled}
        >
          <Clock size={13} />
        </button>
        <button
          type="button"
          className="editor-shell-ai-chat-header-btn"
          onClick={() => { void input.onStartNewSession() }}
          title={ai.panel.newSession}
          aria-label={ai.panel.newSession}
          disabled={input.running || input.sessionLoading || input.canStartNewSession === false}
        >
          <Plus size={13} />
        </button>
      </div>
    </div>
  )
}
