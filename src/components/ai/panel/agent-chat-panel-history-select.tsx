import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/magic-ui/components'
import type { AgentSessionMeta } from '@/lib/agent-chat/session'

import { useAiTranslations } from '../ai-hooks'

function sessionLabel(item: AgentSessionMeta, fallback: string, stateLabel?: string) {
  const title = item.title?.trim()
  const base = title || `${fallback} · ${new Date(item.created_at).toLocaleString()}`

  if (!stateLabel) {
    return base
  }

  return `${base} · ${stateLabel}`
}

function renderSessionItems(input: {
  sessionList: AgentSessionMeta[]
  historyEmpty: string
  historyLabel: string
  historyStateBySessionId?: Record<string, string>
}) {
  if (input.sessionList.length === 0) {
    return <SelectItem value="__none__" disabled>{input.historyEmpty}</SelectItem>
  }

  return input.sessionList.map((item) => (
    <SelectItem key={item.session_id} value={item.session_id}>
      {sessionLabel(item, input.historyLabel, input.historyStateBySessionId?.[item.session_id])}
    </SelectItem>
  ))
}

export function AgentChatPanelHistorySelect(input: {
  running: boolean
  sessionLoading: boolean
  sessionList: AgentSessionMeta[]
  currentSessionId?: string
  historyLabel: string
  historyEmpty: string
  historyStateBySessionId?: Record<string, string>
  onResumeSession: (sessionId: string) => Promise<void>
  onOpenDeleteDialog: (sessionId: string) => void
}) {
  const ai = useAiTranslations()
  const canDelete = Boolean(input.currentSessionId) && !input.running && !input.sessionLoading

  return (
    <div className="flex items-center gap-1">
      <div className="w-36">
        <Select
          value={input.currentSessionId || ''}
          onValueChange={(value) => {
            if (value && value !== '__none__') {
              void input.onResumeSession(value)
            }
          }}
        >
          <SelectTrigger size="sm" disabled={input.running || input.sessionLoading}>
            <SelectValue placeholder={input.historyLabel} />
          </SelectTrigger>
          <SelectContent>
            {renderSessionItems({
              sessionList: input.sessionList,
              historyEmpty: input.historyEmpty,
              historyLabel: input.historyLabel,
              historyStateBySessionId: input.historyStateBySessionId,
            })}
          </SelectContent>
        </Select>
      </div>

      <button
        type="button"
        className="h-7 px-2 rounded border text-[11px] text-secondary-foreground hover-bg disabled:cursor-not-allowed disabled:opacity-50"
        disabled={!canDelete}
        onClick={() => {
          if (!input.currentSessionId) {
            return
          }
          input.onOpenDeleteDialog(input.currentSessionId)
        }}
        aria-label={ai.action.deleteSession}
        title={ai.action.deleteSession}
      >
        {ai.action.deleteSession}
      </button>
    </div>
  )
}
