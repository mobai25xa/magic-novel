import {
  addPinnedAssetToChapter,
  createChapter,
  createVolume,
  getProjectTree,
  readChapter,
  setChapterWordGoal as updateChapterWordGoal,
} from '@/features/editor-reading'
import { useAgentChatStore } from '@/state/agent'
import { useEditorStore } from '@/state/editor'
import type { LeftPanelViewProps } from './left-panel-view'
import { convertFileNode } from './left-panel-types'

type ToastVariant = 'default' | 'success' | 'warning' | 'destructive' | 'info'

type AddToast = (toast: { title: string; description?: string; variant?: ToastVariant }) => void

export type InputDialogState = LeftPanelViewProps['inputDialog']
export type SelectVolumeDialogState = LeftPanelViewProps['selectVolumeDialog']

export function createRefreshTree(input: {
  projectPath: string | null
  setTree: (tree: Array<ReturnType<typeof convertFileNode>>) => void
}) {
  return async () => {
    if (!input.projectPath) return

    try {
      const newTree = await getProjectTree(input.projectPath)
      input.setTree(newTree.map(convertFileNode))
    } catch (error) {
      console.error('Failed to refresh tree:', error)
    }
  }
}

export function createHandleChapterSelect(input: {
  projectPath: string | null
  setCurrentChapter: (chapterId: string, chapterPath: string, title: string) => void
  setContent: (content: unknown) => void
  setIsDirty: (dirty: boolean) => void
}) {
  return async (chapterPath: string, chapterId: string, title?: string) => {
    if (!input.projectPath) return

    const { isDirty } = useEditorStore.getState()
    if (isDirty) {
      try {
        const maybeWindow = window as Window & { __manualSave?: () => Promise<void> }
        const manualSave = maybeWindow.__manualSave
        if (manualSave) await manualSave()
      } catch (error) {
        console.error('Failed to auto-save before switch:', error)
      }
    }

    try {
      const chapter = await readChapter(input.projectPath, chapterPath)
      input.setCurrentChapter(chapterId, chapterPath, title || chapter.title)
      input.setContent(chapter.content)
      input.setIsDirty(false)
      useEditorStore.getState().setLastOpened(input.projectPath, chapterPath, chapterId, title || chapter.title)
      useAgentChatStore.getState().setActiveChapterPath(chapterPath)
    } catch (error) {
      console.error('Failed to read chapter:', error)
    }
  }
}

export function createHandleCreateVolume(input: {
  projectPath: string | null
  translations: { editor: { newVolume: string; enterVolumeName: string }; home: { createSuccess: string; projectCreatedMsg: string; createFailed: string } }
  setInputDialog: (value: InputDialogState) => void
  addToast: AddToast
  refreshTree: () => Promise<void>
}) {
  return async () => {
    if (!input.projectPath) return

    input.setInputDialog({
      open: true,
      title: input.translations.editor.newVolume,
      placeholder: input.translations.editor.enterVolumeName,
      onConfirm: async (title: string) => {
        try {
          await createVolume(input.projectPath!, title)
          await input.refreshTree()
          input.addToast({
            title: input.translations.home.createSuccess,
            description: `${input.translations.home.projectCreatedMsg}${title}`,
            variant: 'success',
          })
        } catch (error) {
          console.error('Failed to create volume:', error)
          input.addToast({
            title: input.translations.home.createFailed,
            description: String(error),
            variant: 'destructive',
          })
        }
      },
    })
  }
}

export function createHandleCreateChapter(input: {
  projectPath: string | null
  treeLength: number
  translations: {
    editor: {
      cannotCreateChapter: string
      createVolumeFirst: string
      newChapter: string
      enterChapterName: string
    }
  }
  addToast: AddToast
  setInputDialog: (value: InputDialogState) => void
  setSelectVolumeDialog: (value: SelectVolumeDialogState) => void
  onCreateVolume: () => Promise<void>
}) {
  return async () => {
    if (!input.projectPath) return

    if (input.treeLength === 0) {
      input.addToast({
        title: input.translations.editor.cannotCreateChapter,
        description: input.translations.editor.createVolumeFirst,
        variant: 'warning',
      })
      await input.onCreateVolume()
      return
    }

    input.setInputDialog({
      open: true,
      title: input.translations.editor.newChapter,
      placeholder: input.translations.editor.enterChapterName,
      onConfirm: (title: string) => {
        input.setSelectVolumeDialog({ open: true, chapterTitle: title })
      },
    })
  }
}

export function createHandleCreateChapterInVolume(input: {
  projectPath: string | null
  translations: { home: { createSuccess: string; projectCreatedMsg: string; createFailed: string } }
  addToast: AddToast
  refreshTree: () => Promise<void>
}) {
  return async (volumePath: string, chapterTitle: string) => {
    if (!input.projectPath) return

    try {
      await createChapter(input.projectPath, volumePath, chapterTitle)
      await input.refreshTree()
      input.addToast({
        title: input.translations.home.createSuccess,
        description: `${input.translations.home.projectCreatedMsg}${chapterTitle}`,
        variant: 'success',
      })
    } catch (error) {
      console.error('Failed to create chapter:', error)
      input.addToast({
        title: input.translations.home.createFailed,
        description: String(error),
        variant: 'destructive',
      })
    }
  }
}

export function createHandlePinAsset(input: {
  projectPath: string | null
  currentChapterPath: string | null
  addToast: AddToast
}) {
  return async (value: string) => {
    if (!input.projectPath || !input.currentChapterPath) return

    try {
      const result = await addPinnedAssetToChapter(input.projectPath, input.currentChapterPath, value)
      if (result.status === 'invalid') return

      if (result.status === 'duplicate') {
        input.addToast({ title: '已存在', description: '该资产已绑定到当前章节', variant: 'info' })
        return
      }

      input.addToast({ title: '绑定成功', description: '已绑定知识库资产到当前章节', variant: 'success' })
    } catch (error) {
      console.error('Failed to pin asset:', error)
      input.addToast({ title: '绑定失败', description: String(error), variant: 'destructive' })
    }
  }
}

export function createHandleWordGoalChange(input: {
  projectPath: string | null
  currentChapterPath: string | null
  translations: {
    editor: {
      goalSetSuccess: string
      goalSetTo: string
      goalCleared: string
      goalSetFailed: string
    }
  }
  setChapterWordGoal: (goal: number | null) => void
  addToast: AddToast
}) {
  return async (newGoal: number | null) => {
    if (!input.projectPath || !input.currentChapterPath) return

    try {
      await updateChapterWordGoal(input.projectPath, input.currentChapterPath, newGoal)
      input.setChapterWordGoal(newGoal)
      input.addToast({
        title: input.translations.editor.goalSetSuccess,
        description: newGoal ? `${input.translations.editor.goalSetTo} ${newGoal}` : input.translations.editor.goalCleared,
        variant: 'success',
      })
    } catch (error) {
      console.error('Failed to set chapter word goal:', error)
      input.addToast({
        title: input.translations.editor.goalSetFailed,
        description: String(error),
        variant: 'destructive',
      })
    }
  }
}

export function createTocSortConfirmHandler(input: {
  setTocSort: (value: { field: 'manual' | 'name' | 'createdAt' | 'updatedAt'; order: 'asc' | 'desc' }) => void
  setTocSortDialogOpen: (open: boolean) => void
}) {
  return (value: string) => {
    const [fieldRaw, orderRaw] = String(value).split(':')
    const field =
      fieldRaw === 'manual' || fieldRaw === 'name' || fieldRaw === 'createdAt' || fieldRaw === 'updatedAt'
        ? fieldRaw
        : 'manual'
    const order = orderRaw === 'desc' ? 'desc' : 'asc'
    input.setTocSort({ field, order })
    input.setTocSortDialogOpen(false)
  }
}
