import { useEditorStore } from '@/state/editor'
import { loadPinnedAssetOptions, handleLeftPanelAssetSelect } from './left-panel-hooks'
import type { LeftPanelViewProps } from './left-panel-view'

type Input = {
  projectExists: boolean
  projectPath: string | null
  currentChapterPath: string | null
  translations: LeftPanelViewProps['translations']
  tree: LeftPanelViewProps['tree']
  inputDialog: LeftPanelViewProps['inputDialog']
  selectVolumeDialog: LeftPanelViewProps['selectVolumeDialog']
  pinnedAssetsDialogOpen: boolean
  pinnedAssetsOptions: { value: string; label: string }[]
  pinnedAssetsDefault?: string
  knowledgeDialog: LeftPanelViewProps['knowledgeDialog']
  setInputDialog: (value: LeftPanelViewProps['inputDialog']) => void
  setSelectVolumeDialog: (value: LeftPanelViewProps['selectVolumeDialog']) => void
  setPinnedAssetsDialogOpen: (open: boolean) => void
  setPinnedAssetsOptions: (value: { value: string; label: string }[]) => void
  setPinnedAssetsDefault: (value: string | undefined) => void
  setKnowledgeDialog: (value: LeftPanelViewProps['knowledgeDialog']) => void
  addToast: (input: { title: string; description?: string; variant?: 'default' | 'success' | 'warning' | 'destructive' | 'info' }) => void
  onCreateVolume: () => void
  onCreateChapter: () => void
  onCreateKnowledgeFolder: () => void
  onCreateKnowledgeFile: () => void
  onChapterSelect: (path: string, id: string, title?: string) => void
  onCreateChapterInVolume: (volumePath: string, chapterTitle: string) => Promise<void>
  onConfirmPinnedAssetsDialog: (value: string) => Promise<void>
}

export function useLeftPanelViewProps(input: Input): LeftPanelViewProps {
  return {
    projectExists: input.projectExists,
    projectPath: input.projectPath,
    translations: input.translations,
    tree: input.tree,
    inputDialog: input.inputDialog,
    selectVolumeDialog: input.selectVolumeDialog,
    pinnedAssetsDialogOpen: input.pinnedAssetsDialogOpen,
    pinnedAssetsOptions: input.pinnedAssetsOptions,
    pinnedAssetsDefault: input.pinnedAssetsDefault,
    knowledgeDialog: input.knowledgeDialog,
    onCreateVolume: input.onCreateVolume,
    onCreateChapter: input.onCreateChapter,
    onCreateKnowledgeFolder: input.onCreateKnowledgeFolder,
    onCreateKnowledgeFile: input.onCreateKnowledgeFile,
    onCreateChapterInVolumeDirect: (volumePath) => {
      input.setInputDialog({
        open: true,
        title: input.translations.editor.newChapter,
        placeholder: input.translations.editor.enterChapterName,
        onConfirm: (title) => {
          void input.onCreateChapterInVolume(volumePath, title)
        },
      })
    },
    onChapterSelect: input.onChapterSelect,
    onAssetSelect: async (relativePath) => {
      if (!input.projectPath) return
      await handleLeftPanelAssetSelect({
        projectPath: input.projectPath,
        relativePath,
        setCurrentAssetDoc: ({ relativePath: path, title, content }) => {
          const editorStore = useEditorStore.getState()
          if (path.startsWith('.magic_novel/')) {
            editorStore.setCurrentKnowledge(path, title || null)
          } else {
            editorStore.setCurrentAsset(path, title || null)
          }
          editorStore.setContent(content)
          editorStore.setIsDirty(false)
        },
      })
    },
    onOpenPinnedAssetsDialog: () => {
      if (!input.projectPath || !input.currentChapterPath) return
      void loadPinnedAssetOptions({
        projectPath: input.projectPath,
        addToast: input.addToast,
        setPinnedAssetsOptions: input.setPinnedAssetsOptions,
        setPinnedAssetsDefault: input.setPinnedAssetsDefault,
        setPinnedAssetsDialogOpen: input.setPinnedAssetsDialogOpen,
      })
    },
    onCloseInputDialog: () => input.setInputDialog(null),
    onClosePinnedAssetsDialog: () => input.setPinnedAssetsDialogOpen(false),
    onConfirmPinnedAssetsDialog: (value) => {
      void input.onConfirmPinnedAssetsDialog(value)
    },
    onCloseSelectVolumeDialog: () => input.setSelectVolumeDialog(null),
    onConfirmSelectVolumeDialog: (volumePath, chapterTitle) => {
      void input.onCreateChapterInVolume(volumePath, chapterTitle)
      input.setSelectVolumeDialog(null)
    },
    onCloseKnowledgeDialog: () => input.setKnowledgeDialog(null),
  }
}
