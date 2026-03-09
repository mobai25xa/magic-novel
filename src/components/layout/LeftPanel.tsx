import { useState } from 'react'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useTranslation } from '@/hooks/use-translation'
import { useToast } from '@/magic-ui/components'
import { LeftPanelView, type LeftPanelViewProps } from './left-panel-view'
import { useLeftPanelViewProps } from './use-left-panel-view-props'
import {
  createHandleChapterSelect,
  createHandleCreateChapter,
  createHandleCreateChapterInVolume,
  createHandleCreateVolume,
  createHandlePinAsset,
  createRefreshTree,
} from './left-panel-actions'
import {
  createHandleCreateKnowledgeFile,
  createHandleCreateKnowledgeFolder,
} from './left-panel-knowledge-actions'

export function LeftPanel() {
  const { project, projectPath, tree, selectedPath, setTree, setSelectedPath } = useProjectStore()
  const { setCurrentChapter, setContent, currentChapterPath, setIsDirty, setCurrentAsset } = useEditorStore()
  const { translations } = useTranslation()
  const { addToast } = useToast()

  const [inputDialog, setInputDialog] = useState<LeftPanelViewProps['inputDialog']>(null)
  const [selectVolumeDialog, setSelectVolumeDialog] = useState<LeftPanelViewProps['selectVolumeDialog']>(null)
  const [pinnedAssetsDialogOpen, setPinnedAssetsDialogOpen] = useState(false)
  const [pinnedAssetsOptions, setPinnedAssetsOptions] = useState<{ value: string; label: string }[]>([])
  const [pinnedAssetsDefault, setPinnedAssetsDefault] = useState<string | undefined>(undefined)
  const [knowledgeDialog, setKnowledgeDialog] = useState<LeftPanelViewProps['knowledgeDialog']>(null)

  const refreshTree = createRefreshTree({ projectPath, setTree })
  const handleChapterSelect = createHandleChapterSelect({ projectPath, setCurrentChapter, setContent, setIsDirty })
  const handleCreateVolume = createHandleCreateVolume({ projectPath, translations, setInputDialog, addToast, refreshTree })
  const handleCreateChapter = createHandleCreateChapter({
    projectPath,
    treeLength: tree.length,
    translations,
    addToast,
    setInputDialog,
    setSelectVolumeDialog,
    onCreateVolume: handleCreateVolume,
  })
  const handleCreateChapterInVolume = createHandleCreateChapterInVolume({ projectPath, translations, addToast, refreshTree })
  const handlePinAsset = createHandlePinAsset({ projectPath, currentChapterPath, addToast })
  const handleCreateKnowledgeFolder = createHandleCreateKnowledgeFolder({
    projectPath,
    tree,
    selectedPath,
    setKnowledgeDialog,
    setProjectTree: setTree,
    addToast,
    translations,
  })
  const handleCreateKnowledgeFile = createHandleCreateKnowledgeFile({
    projectPath,
    tree,
    selectedPath,
    setKnowledgeDialog,
    setProjectTree: setTree,
    addToast,
    translations,
    onCreated: (relativePath, title) => {
      setSelectedPath(`magic_assets/${relativePath}`)
      setCurrentAsset(relativePath, title)
      setContent({
        type: 'doc',
        content: [{ type: 'paragraph', content: [] }],
      })
      setIsDirty(false)
    },
  })

  const viewProps = useLeftPanelViewProps({
    projectExists: Boolean(project),
    projectPath,
    currentChapterPath,
    translations,
    tree,
    inputDialog,
    selectVolumeDialog,
    pinnedAssetsDialogOpen,
    pinnedAssetsOptions,
    pinnedAssetsDefault,
    knowledgeDialog,
    setInputDialog,
    setSelectVolumeDialog,
    setPinnedAssetsDialogOpen,
    setPinnedAssetsOptions,
    setPinnedAssetsDefault,
    setKnowledgeDialog,
    addToast,
    onCreateVolume: handleCreateVolume,
    onCreateChapter: handleCreateChapter,
    onCreateKnowledgeFolder: handleCreateKnowledgeFolder,
    onCreateKnowledgeFile: handleCreateKnowledgeFile,
    onChapterSelect: handleChapterSelect,
    onCreateChapterInVolume: handleCreateChapterInVolume,
    onConfirmPinnedAssetsDialog: async (value) => {
      await handlePinAsset(value)
      setPinnedAssetsDialogOpen(false)
    },
  })

  return <LeftPanelView {...viewProps} />
}

