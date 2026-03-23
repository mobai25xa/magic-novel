import { useEffect, useRef, useState } from 'react'

import {
  AiPanelCardShell,
  AiPanelIconButton,
  AiPanelOverlayShell,
} from '@/magic-ui/components'

import { AgentChatPanelBody } from './message/agent-chat-panel-body'
import { AgentChatPanelInputShell } from './message/agent-chat-panel-input-shell'
import { MissionPanel } from './mission-panel/MissionPanel'
import { useDroppedFramesMetric } from './panel/agent-chat-panel-hooks'
import {
  createInitialChatScrollState,
  handleIncomingContent,
  updateScrollLockState,
} from './panel/agent-chat-panel-scroll'
import {
  AgentChatPanelHeader,
} from './panel/view/agent-chat-panel-view-controls'
import { AgentChatPanelHistoryPage } from './panel/agent-chat-panel-history-page'
import { AgentChatPanelViewStatus } from './panel/view/agent-chat-panel-view-status'
import type { AgentChatPanelViewProps } from './panel/view/agent-chat-panel-view-types'
import { useAgentChatStore } from '@/state/agent'
import { useAiTranslations } from './ai-hooks'

export function AgentChatPanelView(input: AgentChatPanelViewProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const [scrollState, setScrollState] = useState(() => createInitialChatScrollState())
  const [deleteDialogSessionId, setDeleteDialogSessionId] = useState<string | null>(null)
  const todoState = useAgentChatStore((state) => state.todoState)
  const ai = useAiTranslations()
  const hasProject = Boolean(input.projectPath)

  useDroppedFramesMetric({
    sessionId: input.sessionId,
    enabled: input.running,
  })

  useEffect(() => {
    setScrollState((prev) => handleIncomingContent(prev, scrollRef, Boolean(input.latestTurnSignature)))
  }, [input.latestTurnSignature])

  useEffect(() => {
    if (!scrollRef.current) {
      return
    }
    setScrollState((prev) => updateScrollLockState(prev, scrollRef.current!))
  }, [input.running])

  const handleOpenHistoryPage = () => {
    input.onOpenHistoryPage()
  }

  const handleCloseHistoryPage = () => {
    setDeleteDialogSessionId(null)
    input.onCloseHistoryPage()
  }

  return (
    <div className="editor-shell-ai-chat-root">
      <AgentChatPanelHeader
        running={input.running}
        canStartNewSession={input.canStartNewSession}
        onStartNewSession={input.onStartNewSession}
        onToggleHistoryPage={input.historyPageOpen ? handleCloseHistoryPage : handleOpenHistoryPage}
        historyPageOpen={input.historyPageOpen}
        historyEnabled={input.historyEnabled}
        sessionLoading={input.sessionLoading}
        missionDisabled={!hasProject}
        onOpenMissionPanel={input.onOpenMissionPanel}
      />

      <AgentChatPanelViewStatus
        lastError={input.lastError}
        sessionError={input.sessionError}
        wasSessionResumed={input.wasSessionResumed}
        sessionRuntimeState={input.sessionRuntimeState}
        sessionCanContinue={input.sessionCanContinue}
        sessionCanResume={input.sessionCanResume}
        sessionReadonlyReason={input.sessionReadonlyReason}
        sessionHydrationStatus={input.sessionHydrationStatus}
        sessionWarnings={input.sessionWarnings}
      />

      <AgentChatPanelBody
        turnIds={input.turnIds}
        viewMode={input.viewMode}
        sessionId={input.sessionId}
        running={input.running}
        sessionRuntimeState={input.sessionRuntimeState}
        sessionCanResume={input.sessionCanResume}
        sessionReadonlyReason={input.sessionReadonlyReason}
        onRetryStep={input.onRetryStep}
        scrollRef={scrollRef}
        scrollState={scrollState}
        setScrollState={setScrollState}
      />

      <AgentChatPanelInputShell
        running={input.running}
        input={input.input}
        inputDisabled={input.inputDisabled}
        inputPlaceholder={input.inputPlaceholder}
        sessionCanContinue={input.sessionCanContinue}
        onInputChange={input.onInputChange}
        onSend={input.onSend}
        models={input.models}
        selectedModel={input.selectedModel}
        onSelectModel={input.onSelectModel}
        approvalMode={input.approvalMode}
        onCancel={input.onCancel}
        elapsedTime={input.elapsedTime}
        elapsedSeconds={input.elapsedSeconds}
        showTimer={input.showTimer}
        todoState={todoState}
      />

      <AgentChatPanelHistoryPage
        open={input.historyPageOpen}
        running={input.running}
        loading={input.sessionLoading}
        sessionList={input.sessionList}
        currentSessionId={input.currentSessionId}
        historyStateBySessionId={input.historyStateBySessionId}
        deleteDialogOpen={Boolean(deleteDialogSessionId)}
        deleteSessionId={deleteDialogSessionId ?? undefined}
        onOpenDeleteDialog={(sessionId) => setDeleteDialogSessionId(sessionId)}
        onCloseDeleteDialog={() => setDeleteDialogSessionId(null)}
        onClose={handleCloseHistoryPage}
        onResumeSession={input.onResumeSession}
        onRenameSession={input.onRenameSession}
        onDeleteSession={input.onDeleteSession}
        onOpenMissionPanel={input.onOpenMissionPanel}
      />

      {input.missionPanelOpen && hasProject && (
        <AiPanelOverlayShell className="p-3 overflow-auto">
          {input.missionId ? (
            <MissionPanel
              projectPath={input.projectPath}
              missionId={input.missionId}
              onClose={input.onCloseMissionPanel}
            />
          ) : (
            <AiPanelCardShell className="p-4 bg-card">
              <div className="flex items-center justify-between">
                <span className="font-semibold text-foreground">{ai.panel.mission}</span>
                <AiPanelIconButton
                  onClick={input.onCloseMissionPanel}
                  title={ai.action.closeHistoryPage}
                >
                  ✕
                </AiPanelIconButton>
              </div>
              <p className="text-xs text-muted-foreground">{ai.panel.missionEmpty}</p>
            </AiPanelCardShell>
          )}
        </AiPanelOverlayShell>
      )}
    </div>
  )
}
