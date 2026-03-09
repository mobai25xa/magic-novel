import { Flag, History, Plus } from 'lucide-react'

import { Button } from '@/magic-ui/components'

function HistoryButton(input: {
  enabled: boolean
  active: boolean
  disabledLabel: string
  historyLabel: string
  onToggle: () => void
  disabled?: boolean
}) {
  if (!input.enabled) {
    return (
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="h-7 w-7 opacity-70 cursor-not-allowed"
        title={input.disabledLabel}
        aria-label={input.disabledLabel}
        disabled
      >
        <History className="h-4 w-4" />
      </Button>
    )
  }

  return (
    <Button
      type="button"
      onClick={input.onToggle}
      variant="ghost"
      size="icon"
      className={`h-7 w-7 ${input.active ? 'active-bg border-primary' : ''}`}
      title={input.historyLabel}
      aria-label={input.historyLabel}
      disabled={input.disabled}
    >
      <History className="h-4 w-4" />
    </Button>
  )
}

function NewSessionButton(input: {
  running: boolean
  sessionLoading: boolean
  onStartNewSession: () => Promise<void>
  title: string
}) {
  return (
    <Button
      type="button"
      onClick={() => { void input.onStartNewSession() }}
      variant="ghost"
      size="icon"
      className="h-7 w-7"
      disabled={input.running || input.sessionLoading}
      aria-label={input.title}
      title={input.title}
    >
      <Plus className="h-4 w-4" />
    </Button>
  )
}

export function HeaderSideActions(input: {
  historyEnabled: boolean
  historyUnavailable: string
  historyLabel: string
  historyPageOpen: boolean
  running: boolean
  sessionLoading: boolean
  onToggleHistoryPage: () => void
  onStartNewSession: () => Promise<void>
  newSessionLabel: string
  missionLabel: string
  missionHint: string
  missionDisabled: boolean
  onOpenMissionPanel: () => void
}) {
  return (
    <div className="flex items-center gap-1">
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="h-7 w-7 disabled:opacity-70 disabled:cursor-not-allowed"
        onClick={input.onOpenMissionPanel}
        disabled={input.missionDisabled}
        aria-label={input.missionLabel}
        title={input.missionDisabled ? input.missionHint : input.missionLabel}
      >
        <Flag className="h-4 w-4" />
      </Button>
      <HistoryButton
        enabled={input.historyEnabled}
        active={input.historyPageOpen}
        disabledLabel={input.historyUnavailable}
        historyLabel={input.historyLabel}
        onToggle={input.onToggleHistoryPage}
        disabled={input.sessionLoading}
      />
      <NewSessionButton
        running={input.running}
        sessionLoading={input.sessionLoading}
        onStartNewSession={input.onStartNewSession}
        title={input.newSessionLabel}
      />
    </div>
  )
}
