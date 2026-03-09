import { useCallback, useEffect, useRef, type RefObject } from 'react'

import { readChapter } from '@/features/editor-reading'
import { useAgentChatStore } from '@/state/agent'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'

import { MAX_PANEL_WIDTH, MIN_PANEL_WIDTH, useLayoutStore, useEditorPanelAutoHide } from '@/stores/layout-store'

import { EditorPanel } from '../layout/EditorPanel'
import { LeftPanel } from '../layout/LeftPanel'
import { ResizableHandle } from '../layout/ResizableHandle'
import { RightPanel } from '../layout/RightPanel'
import { TopBar } from '../layout/TopBar'
import { FullscreenMode } from './FullscreenMode'

interface EditorPageProps {
  onOpenSettings: () => void
}

function clampPanelWidth(width: number) {
  return Math.max(MIN_PANEL_WIDTH, Math.min(MAX_PANEL_WIDTH, width))
}

function applyPanelWidth(panelRef: RefObject<HTMLDivElement | null>, width: number) {
  if (!panelRef.current) return
  panelRef.current.style.width = `${width}px`
}

function calculateRightPanelTopOffset(panelElement: HTMLDivElement | null) {
  if (typeof window === 'undefined' || !panelElement) return 0

  const topbar = document.querySelector('.editor-shell-topbar') as HTMLElement | null
  if (!topbar) return 0

  const topbarRight = document.querySelector('.editor-shell-topbar-right') as HTMLElement | null
  const topbarRect = topbar.getBoundingClientRect()
  const topbarRightRect = topbarRight?.getBoundingClientRect() ?? topbarRect
  const panelRect = panelElement.getBoundingClientRect()

  const horizontalOverlap = topbarRightRect.right - panelRect.left
  if (horizontalOverlap <= 0) return 0

  const appliedOffsetRaw = panelElement.style.getPropertyValue('--editor-shell-right-panel-offset')
  const appliedOffset = Number.parseFloat(appliedOffsetRaw)
  const normalizedAppliedOffset = Number.isFinite(appliedOffset) ? appliedOffset : 0
  const naturalPanelTop = panelRect.top - normalizedAppliedOffset
  const verticalOverlap = Math.round(topbarRect.bottom - naturalPanelTop)

  return Number.isFinite(verticalOverlap) && verticalOverlap > 0 ? verticalOverlap : 0
}

function applyRightPanelTopOffset(panelRef: RefObject<HTMLDivElement | null>, offset: number) {
  if (!panelRef.current) return
  panelRef.current.style.setProperty('--editor-shell-right-panel-offset', `${offset}px`)
}

function useResizablePanels({
  leftPanelWidth,
  rightPanelWidth,
  setLeftPanelWidth,
  setRightPanelWidth,
}: {
  leftPanelWidth: number
  rightPanelWidth: number
  setLeftPanelWidth: (width: number) => void
  setRightPanelWidth: (width: number) => void
}) {
  const leftPanelRef = useRef<HTMLDivElement | null>(null)
  const rightPanelRef = useRef<HTMLDivElement | null>(null)
  const leftWidthRef = useRef(leftPanelWidth)
  const rightWidthRef = useRef(rightPanelWidth)

  const syncRightPanelTopOffset = useCallback(() => {
    applyRightPanelTopOffset(rightPanelRef, calculateRightPanelTopOffset(rightPanelRef.current))
  }, [])

  const attachLeftPanel = useCallback((element: HTMLDivElement | null) => {
    leftPanelRef.current = element
    if (!element) return
    element.style.width = `${leftWidthRef.current}px`
  }, [])

  const attachRightPanel = useCallback((element: HTMLDivElement | null) => {
    rightPanelRef.current = element
    if (!element) return
    element.style.width = `${rightWidthRef.current}px`
    syncRightPanelTopOffset()
  }, [syncRightPanelTopOffset])

  useEffect(() => {
    leftWidthRef.current = leftPanelWidth
    applyPanelWidth(leftPanelRef, leftPanelWidth)
    syncRightPanelTopOffset()
  }, [leftPanelWidth, syncRightPanelTopOffset])

  useEffect(() => {
    rightWidthRef.current = rightPanelWidth
    applyPanelWidth(rightPanelRef, rightPanelWidth)
    syncRightPanelTopOffset()
  }, [rightPanelWidth, syncRightPanelTopOffset])

  useEffect(() => {
    syncRightPanelTopOffset()

    const handleResize = () => {
      syncRightPanelTopOffset()
    }

    window.addEventListener('resize', handleResize)

    return () => {
      window.removeEventListener('resize', handleResize)
    }
  }, [syncRightPanelTopOffset])

  const handleResizeLeft = useCallback((delta: number) => {
    const next = clampPanelWidth(leftWidthRef.current + delta)
    leftWidthRef.current = next
    applyPanelWidth(leftPanelRef, next)
  }, [])

  const handleResizeRight = useCallback((delta: number) => {
    const next = clampPanelWidth(rightWidthRef.current + delta)
    rightWidthRef.current = next
    applyPanelWidth(rightPanelRef, next)
  }, [])

  const handleResizeLeftEnd = useCallback(() => {
    setLeftPanelWidth(leftWidthRef.current)
  }, [setLeftPanelWidth])

  const handleResizeRightEnd = useCallback(() => {
    setRightPanelWidth(rightWidthRef.current)
  }, [setRightPanelWidth])

  return {
    attachLeftPanel,
    attachRightPanel,
    handleResizeLeft,
    handleResizeRight,
    handleResizeLeftEnd,
    handleResizeRightEnd,
  }
}

function useRestoreLastOpenedChapter(projectPath: string | null, currentChapterId: string | null) {
  useEffect(() => {
    const {
      lastOpenedProjectPath,
      lastOpenedChapterPath,
      lastOpenedChapterId,
      lastOpenedChapterTitle,
      setCurrentChapter,
      setContent,
      setIsDirty,
      currentDocKind,
    } = useEditorStore.getState()

    if (
      projectPath &&
      lastOpenedProjectPath === projectPath &&
      lastOpenedChapterPath &&
      lastOpenedChapterId &&
      !currentChapterId &&
      currentDocKind !== 'asset'
    ) {
      readChapter(projectPath, lastOpenedChapterPath)
        .then((chapter) => {
          setCurrentChapter(lastOpenedChapterId, lastOpenedChapterPath, lastOpenedChapterTitle || chapter.title)
          setContent(chapter.content)
          setIsDirty(false)
          useAgentChatStore.getState().setActiveChapterPath(lastOpenedChapterPath)
        })
        .catch((error) => {
          console.error('Failed to restore last opened chapter:', error)
        })
    }
  }, [projectPath, currentChapterId])
}

export function EditorPage({ onOpenSettings }: EditorPageProps) {
  const {
    isLeftPanelVisible,
    isRightPanelVisible,
    leftPanelWidth,
    rightPanelWidth,
    setLeftPanelWidth,
    setRightPanelWidth,
  } = useLayoutStore()
  const { projectPath } = useProjectStore()
  const { currentChapterId } = useEditorStore()
  const stateStatus = useAgentChatStore((state) => state.stateStatus)
  const sessionRuntimeState = useAgentChatStore((state) => state.sessionRuntimeState)
  const isAgentSessionActive = stateStatus !== 'idle'
    || sessionRuntimeState === 'running'
    || sessionRuntimeState === 'suspended_confirmation'
    || sessionRuntimeState === 'suspended_askuser'

  const {
    attachLeftPanel,
    attachRightPanel,
    handleResizeLeft,
    handleResizeRight,
    handleResizeLeftEnd,
    handleResizeRightEnd,
  } = useResizablePanels({
    leftPanelWidth,
    rightPanelWidth,
    setLeftPanelWidth,
    setRightPanelWidth,
  })

  useRestoreLastOpenedChapter(projectPath, currentChapterId)

  // 响应式面板自动隐藏
  useEditorPanelAutoHide({ disableRightPanelAutoHide: isAgentSessionActive })

  return (
    <div className="app-page app-page-editor editor-shell-page">
      <div className="editor-shell-body">
        {isLeftPanelVisible ? (
          <div ref={attachLeftPanel} className="editor-shell-left-wrap">
            <LeftPanel />
            <ResizableHandle
              direction="left"
              onResize={handleResizeLeft}
              onResizeEnd={handleResizeLeftEnd}
            />
          </div>
        ) : null}

        <div className="editor-shell-center">
          <TopBar onOpenSettings={onOpenSettings} />

          <main className="editor-shell-main">
            <FullscreenMode>
              <EditorPanel />
            </FullscreenMode>
          </main>
        </div>

        {isRightPanelVisible ? (
          <div ref={attachRightPanel} className="editor-shell-right-wrap">
            <ResizableHandle
              direction="right"
              onResize={handleResizeRight}
              onResizeEnd={handleResizeRightEnd}
            />
            <RightPanel />
          </div>
        ) : null}
      </div>
    </div>
  )
}
