import { readChapter } from '@/features/editor-reading'
import { refreshProjectTreeLifecycle } from '@/features/project-lifecycle'
import { toast } from '@/magic-ui/components'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'

import { useAgentChatStore } from '../store'
import type { extractToolPreviewRefs } from '../tool-trace'

import { TOOL_RESULT_REFRESH_DEBOUNCE_MS } from './channels'
import type { ToolChangeSet } from './types'
import { asRecord } from './utils'

let pendingToolRefreshTimer: ReturnType<typeof setTimeout> | null = null
let pendingToolChangeSet: ToolChangeSet | null = null

function mergeToolChangeSet(input: ToolChangeSet) {
  if (!pendingToolChangeSet) {
    pendingToolChangeSet = {
      shouldRefreshTree: input.shouldRefreshTree,
      shouldRefreshEditor: input.shouldRefreshEditor,
      chapterPath: input.chapterPath,
      projectPath: input.projectPath,
    }
    return
  }

  pendingToolChangeSet = {
    shouldRefreshTree: pendingToolChangeSet.shouldRefreshTree || input.shouldRefreshTree,
    shouldRefreshEditor: pendingToolChangeSet.shouldRefreshEditor || input.shouldRefreshEditor,
    chapterPath: input.chapterPath || pendingToolChangeSet.chapterPath,
    projectPath: input.projectPath || pendingToolChangeSet.projectPath,
  }
}

function extractToolChapterPath(input: {
  payload: Record<string, unknown>
  traceRefs: ReturnType<typeof extractToolPreviewRefs>
}) {
  const fromNewPath = typeof input.traceRefs?.path === 'string' ? input.traceRefs.path : undefined
  if (fromNewPath && fromNewPath.trim()) {
    return fromNewPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')
  }

  const fromArgs = asRecord(input.payload.args)
  const targetRef = typeof fromArgs?.target_ref === 'string' ? fromArgs.target_ref : undefined
  const fromTargetRef = targetRef
    ? targetRef.trim().replace(/\\/g, '/').replace(/^chapter:/, '')
    : undefined
  if (fromTargetRef && fromTargetRef.trim()) {
    return fromTargetRef.replace(/^manuscripts\//, '')
  }

  const chapterPath = typeof fromArgs?.chapter_path === 'string'
    ? fromArgs.chapter_path
    : typeof fromArgs?.path === 'string'
      ? fromArgs.path
      : undefined

  if (chapterPath && chapterPath.trim()) {
    return chapterPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')
  }

  const activeChapterPath = useAgentChatStore.getState().active_chapter_path
  return activeChapterPath && activeChapterPath.trim()
    ? activeChapterPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')
    : undefined
}

async function applyToolRefresh(changeSet: ToolChangeSet) {
  const projectPath = (changeSet.projectPath || useProjectStore.getState().projectPath || '').trim()
  if (!projectPath) {
    return
  }

  if (changeSet.shouldRefreshTree) {
    try {
      const tree = await refreshProjectTreeLifecycle(projectPath)
      useProjectStore.getState().setTree(tree)
    } catch (error) {
      console.error('[agent-event] refresh tree failed:', error)
    }
  }

  if (changeSet.shouldRefreshEditor && changeSet.chapterPath) {
    const chapterPath = changeSet.chapterPath.trim()
    const editorStore = useEditorStore.getState()
    if (!chapterPath || editorStore.currentChapterPath !== chapterPath) {
      return
    }

    if (editorStore.isDirty) {
      const projectPathForPending = (changeSet.projectPath || useProjectStore.getState().projectPath || '').trim()
      const existing = editorStore.pendingExternalChapterRefresh
      const nextPending = {
        projectPath: projectPathForPending,
        chapterPath,
        requestedAt: Date.now(),
      }

      if (
        !existing
        || existing.chapterPath !== nextPending.chapterPath
        || existing.projectPath !== nextPending.projectPath
      ) {
        editorStore.setPendingExternalChapterRefresh(nextPending)
        toast.info('内容已更新', '检测到 AI 写入了当前章节，但你有未保存的修改；已暂停自动刷新。')
      }

      return
    }

    try {
      const chapter = await readChapter(projectPath, chapterPath)
      useProjectStore.getState().setSelectedPath(chapterPath)
      editorStore.setCurrentChapter(chapter.id, chapterPath, chapter.title)
      editorStore.setContent(chapter.content)
      editorStore.setIsDirty(false)
      editorStore.clearPendingExternalChapterRefresh()
      editorStore.setLastOpened(projectPath, chapterPath, chapter.id, chapter.title)
      useAgentChatStore.getState().setActiveChapterPath(chapterPath)
    } catch (error) {
      console.error('[agent-event] refresh editor failed:', error)
    }
  }
}

export function scheduleToolRefresh(changeSet: ToolChangeSet) {
  mergeToolChangeSet(changeSet)

  if (pendingToolRefreshTimer) {
    return
  }

  pendingToolRefreshTimer = setTimeout(() => {
    const next = pendingToolChangeSet
    pendingToolRefreshTimer = null
    pendingToolChangeSet = null

    if (!next) {
      return
    }

    void applyToolRefresh(next)
  }, TOOL_RESULT_REFRESH_DEBOUNCE_MS)
}

export function buildToolRefreshChangeSet(input: {
  toolName: string
  status: string
  payload: Record<string, unknown>
  tracePreview: Record<string, unknown>
  traceRefs: ReturnType<typeof extractToolPreviewRefs>
}): ToolChangeSet | null {
  if (input.status !== 'ok') {
    return null
  }

  const toolName = input.toolName.trim().toLowerCase()
  if (
    toolName !== 'structure_edit'
    && toolName !== 'draft_write'
    && toolName !== 'knowledge_write'
    && toolName !== 'create'
    && toolName !== 'edit'
    && toolName !== 'delete'
    && toolName !== 'move'
  ) {
    return null
  }

  const chapterPath = extractToolChapterPath({
    payload: input.payload,
    traceRefs: input.traceRefs,
  })
  const shouldRefreshEditor = toolName === 'draft_write'
    || toolName === 'edit'
    || (toolName === 'structure_edit' && Boolean(chapterPath))
    || (toolName === 'move' && Boolean(chapterPath))

  return {
    shouldRefreshTree: true,
    shouldRefreshEditor,
    chapterPath,
  }
}
