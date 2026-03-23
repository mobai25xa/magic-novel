import { useEffect, useMemo, useRef } from 'react'
import debounce from 'lodash.debounce'

import { useAgentChatStore } from '@/state/agent'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useEditorUiStore } from '@/stores/editor-ui-store'

import {
  applySessionUiState,
  buildSessionUiStateSnapshot,
  loadSessionUiStateFromDisk,
  saveSessionUiStateToDisk,
} from './session-ui-state'

const SAVE_DEBOUNCE_MS = 500

export function useSessionUiStateSync() {
  const projectPath = useProjectStore((state) => state.projectPath)
  const selectedPath = useProjectStore((state) => state.selectedPath)

  const leftTab = useEditorUiStore((state) => state.leftPanelTab)
  const collapsedDirPaths = useEditorUiStore((state) => state.sidebarTreeCollapsedDirPaths)

  const currentDocKind = useEditorStore((state) => state.currentDocKind)
  const currentChapterPath = useEditorStore((state) => state.currentChapterPath)
  const currentAssetPath = useEditorStore((state) => state.currentAssetPath)

  const sessionId = useAgentChatStore((state) => state.session_id)
  const persistedSessionId = useAgentChatStore((state) => state.currentSessionMeta?.session_id)
  const wasSessionResumed = useAgentChatStore((state) => state.wasSessionResumed)
  const consumeWasSessionResumed = useAgentChatStore((state) => state.consumeWasSessionResumed)

  const activeChapterPath = useAgentChatStore((state) => state.active_chapter_path)

  const canPersist = Boolean(projectPath)
    && Boolean(sessionId)
    && Boolean(persistedSessionId)
    && persistedSessionId === sessionId

  const scheduleSave = useMemo(() => {
    return debounce(async (snapshot: { projectPath: string; sessionId: string }) => {
      try {
        const uiState = buildSessionUiStateSnapshot()
        await saveSessionUiStateToDisk({
          projectPath: snapshot.projectPath,
          sessionId: snapshot.sessionId,
          uiState,
        })
      } catch (error) {
        console.warn('[session-ui-state] Failed to persist ui_state:', error)
      }
    }, SAVE_DEBOUNCE_MS)
  }, [])

  const lastSessionKeyRef = useRef<string | null>(null)
  useEffect(() => {
    if (!projectPath || !sessionId) {
      return
    }

    const nextKey = `${projectPath}::${sessionId}`
    if (!lastSessionKeyRef.current) {
      lastSessionKeyRef.current = nextKey
      return
    }

    if (lastSessionKeyRef.current === nextKey) {
      return
    }

    lastSessionKeyRef.current = nextKey
    useEditorUiStore.getState().resetSessionUiState()
  }, [projectPath, sessionId])

  useEffect(() => {
    if (!projectPath || !sessionId) {
      return () => {
        scheduleSave.flush()
        scheduleSave.cancel()
      }
    }

    if (!canPersist) {
      return () => {
        scheduleSave.flush()
        scheduleSave.cancel()
      }
    }

    if (wasSessionResumed) {
      return () => {
        scheduleSave.flush()
        scheduleSave.cancel()
      }
    }

    scheduleSave({ projectPath, sessionId })

    return () => {
      scheduleSave.flush()
      scheduleSave.cancel()
    }
  }, [
    projectPath,
    sessionId,
    canPersist,
    wasSessionResumed,
    selectedPath,
    leftTab,
    collapsedDirPaths,
    currentDocKind,
    currentChapterPath,
    currentAssetPath,
    activeChapterPath,
    scheduleSave,
  ])

  useEffect(() => {
    if (!wasSessionResumed) {
      return
    }

    if (!projectPath || !sessionId) {
      return
    }

    const consumed = consumeWasSessionResumed()
    if (!consumed) {
      return
    }

    void (async () => {
      const restored = await loadSessionUiStateFromDisk({ projectPath, sessionId })
      if (restored) {
        await applySessionUiState(restored)
      }
    })()
  }, [consumeWasSessionResumed, projectPath, sessionId, wasSessionResumed])
}
