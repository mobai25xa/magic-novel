import { useCallback, useMemo } from 'react'

import { readChapter } from '@/features/editor-reading'
import { Button, toast } from '@/magic-ui/components'
import { useAgentChatStore } from '@/state/agent'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'

export function PendingExternalChapterRefreshBanner() {
  const projectPath = useProjectStore((state) => state.projectPath)
  const pendingRefresh = useEditorStore((state) => state.pendingExternalChapterRefresh)
  const currentDocKind = useEditorStore((state) => state.currentDocKind)
  const currentChapterPath = useEditorStore((state) => state.currentChapterPath)
  const isDirty = useEditorStore((state) => state.isDirty)

  const shouldShow = useMemo(() => {
    if (!projectPath || !pendingRefresh) return false
    if (!isDirty || currentDocKind !== 'chapter') return false
    if (!currentChapterPath || currentChapterPath !== pendingRefresh.chapterPath) return false
    if (pendingRefresh.projectPath && pendingRefresh.projectPath !== projectPath) return false
    return true
  }, [currentChapterPath, currentDocKind, isDirty, pendingRefresh, projectPath])

  const handleIgnore = useCallback(() => {
    useEditorStore.getState().clearPendingExternalChapterRefresh()
    toast.default('已忽略更新', '将保留你当前未保存的编辑内容。')
  }, [])

  const handleRefresh = useCallback(async () => {
    const projectPathFromStore = useProjectStore.getState().projectPath
    const editorStore = useEditorStore.getState()
    const pending = editorStore.pendingExternalChapterRefresh

    if (!projectPathFromStore || !pending?.chapterPath) {
      return
    }

    try {
      const chapter = await readChapter(projectPathFromStore, pending.chapterPath)
      useProjectStore.getState().setSelectedPath(pending.chapterPath)
      editorStore.setCurrentChapter(chapter.id, pending.chapterPath, chapter.title)
      editorStore.setContent(chapter.content)
      editorStore.setIsDirty(false)
      editorStore.clearPendingExternalChapterRefresh()
      editorStore.setLastOpened(projectPathFromStore, pending.chapterPath, chapter.id, chapter.title)
      useAgentChatStore.getState().setActiveChapterPath(pending.chapterPath)

      toast.success('已刷新', '编辑器内容已更新为 AI 写入后的最新版本。')
    } catch (error) {
      console.error('[editor] refresh pending chapter failed:', error)
      toast.error('刷新失败', '读取章节内容失败，请稍后重试。')
    }
  }, [])

  if (!shouldShow) {
    return null
  }

  return (
    <div className="mx-4 mt-3 rounded-md border bg-muted/40 px-3 py-2 text-sm">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <div className="font-medium">内容已更新</div>
          <div className="text-xs text-muted-foreground">
            AI 已写入当前章节；因你有未保存修改，已暂停自动刷新。选择刷新将覆盖当前编辑内容。
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Button type="button" variant="secondary" size="sm" onClick={handleIgnore}>
            忽略
          </Button>
          <Button type="button" size="sm" onClick={() => void handleRefresh()}>
            刷新
          </Button>
        </div>
      </div>
    </div>
  )
}

