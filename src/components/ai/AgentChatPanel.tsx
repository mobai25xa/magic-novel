import { useState } from 'react'

import { createChatPanelActions } from './agent-chat-panel-actions'
import { AgentChatPanelView } from './agent-chat-panel-view'
import { useAgentChatPanelState } from './panel/agent-chat-panel-state'

type AgentChatPanelProps = {
  onClosePanel?: () => void
}

export function AgentChatPanel({ onClosePanel }: AgentChatPanelProps) {
  const state = useAgentChatPanelState()
  const [missionPanelOpen, setMissionPanelOpen] = useState(false)

  const {
    handleSend,
    handleRetryStep,
    handleCancel,
  } = createChatPanelActions({
    approvalMode: state.approvalMode,
    capabilityMode: state.capabilityMode,
    running: state.running,
    missionBusy: state.missionBusy,
    missionLocked: state.missionLocked,
    missionId: state.missionId,
    projectPath: state.projectPath,
    canContinue: state.sessionCanContinue,
    runtimeState: state.sessionRuntimeState,
    setInput: state.setInput,
    setLastError: state.setLastError,
    setMissionDispatching: state.setMissionDispatching,
    onMissionStarted: () => setMissionPanelOpen(true),
    contexts: state.contexts,
    clearContexts: state.clearContexts,
    labels: state.chatLabels,
  })

  return (
    <AgentChatPanelView
      sessionId={state.sessionId}
      turnIds={state.turnIds}
      latestTurnSignature={state.latestTurnSignature}
      models={state.availableModels}
      selectedModel={state.selectedModel}
      onSelectModel={state.handleSelectModel}
      approvalMode={state.approvalMode}
      capabilityMode={state.capabilityMode}
      viewMode={state.aiChatViewMode}
      onSelectViewMode={state.setAiChatViewMode}
      input={state.input}
      running={state.running}
      lastError={state.lastError}
      sessionError={state.sessionError}
      wasSessionResumed={state.wasSessionResumed}
      sessionRuntimeState={state.sessionRuntimeState}
      sessionHydrationStatus={state.sessionHydrationStatus}
      sessionCanContinue={state.sessionCanContinue}
      sessionCanResume={state.sessionCanResume}
      sessionReadonlyReason={state.sessionReadonlyReason}
      sessionWarnings={state.sessionWarnings}
      historyStateBySessionId={state.historyStateBySessionId}
      inputDisabled={state.sessionInputDisabled}
      inputPlaceholder={state.sessionInputPlaceholder}
      onInputChange={state.setInput}
      onSend={() => handleSend(state.input)}
      onCancel={handleCancel}
      canStartNewSession={state.canStartNewSession}
      onStartNewSession={state.startNewSession}
      onOpenHistoryPage={state.openHistoryPage}
      onCloseHistoryPage={state.closeHistoryPage}
      historyPageOpen={state.historyPageOpen}
      onResumeSession={state.resumeSession}
      onRenameSession={state.renameSession}
      onDeleteSession={state.deleteSession}
      historyEnabled={state.sessionPersistenceEnabled}
      sessionLoading={state.isSessionLoading}
      sessionList={state.sessionList}
      currentSessionId={state.currentSessionMeta?.session_id}
      onRetryStep={handleRetryStep}
      contexts={state.contexts}
      onAddContext={state.addContext}
      onRemoveContext={state.removeContext}
      elapsedTime={state.streamingElapsedTime}
      elapsedSeconds={state.streamingElapsedSeconds}
      showTimer={state.showStreamingTimer}
      projectPath={state.projectPath}
      missionId={state.missionId}
      missionPanelOpen={missionPanelOpen}
      onOpenMissionPanel={() => setMissionPanelOpen(true)}
      onCloseMissionPanel={() => setMissionPanelOpen(false)}
      onClosePanel={onClosePanel}
    />
  )
}
