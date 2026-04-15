import { useState } from 'react'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useTranslation } from '@/hooks/use-translation'
import { useToast } from '@/magic-ui/components'
import { openEditorTarget } from '@/features/editor-navigation/open-editor-target'
import { refreshPlanningManifestEntry } from '@/features/project-home'
import { resolveRecommendedPlanningTarget } from '@/features/project-home/planning-manifest-helpers'
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
  const {
    project,
    projectPath,
    tree,
    selectedPath,
    planningManifest,
    planningManifestProjectPath,
    setTree,
    setSelectedPath,
  } = useProjectStore()
  const { setCurrentChapter, setContent, currentChapterPath, setIsDirty } = useEditorStore()
  const { translations } = useTranslation()
  const { addToast } = useToast()

  const [inputDialog, setInputDialog] = useState<LeftPanelViewProps['inputDialog']>(null)
  const [selectVolumeDialog, setSelectVolumeDialog] = useState<LeftPanelViewProps['selectVolumeDialog']>(null)
  const [pinnedAssetsDialogOpen, setPinnedAssetsDialogOpen] = useState(false)
  const [pinnedAssetsOptions, setPinnedAssetsOptions] = useState<{ value: string; label: string }[]>([])
  const [pinnedAssetsDefault, setPinnedAssetsDefault] = useState<string | undefined>(undefined)
  const [knowledgeDialog, setKnowledgeDialog] = useState<LeftPanelViewProps['knowledgeDialog']>(null)

  const activePlanningManifest = projectPath && planningManifestProjectPath === projectPath
    ? planningManifest
    : null

  const mapPlanningBlocker = (blocker: string) => {
    if (blocker === 'narrative_contract_unconfirmed') {
      return translations.projectHome.blockerNarrativeContract
    }

    if (blocker === 'chapter_1_detail_unconfirmed') {
      return translations.projectHome.blockerChapter1Detail
    }

    return blocker
  }

  const guardWritingEntry = async () => {
    if (!projectPath) {
      return false
    }

    let manifest = activePlanningManifest
    if (!manifest) {
      try {
        manifest = await refreshPlanningManifestEntry(projectPath)
        useProjectStore.getState().setPlanningManifest(projectPath, manifest)
      } catch (error) {
        console.error('[project-home] Failed to load planning manifest for write gate:', error)
        addToast({
          title: translations.projectHome.manifestRefreshFailedTitle,
          description: String(error),
          variant: 'destructive',
        })
        return false
      }
    }

    if (manifest.writing_readiness.can_start) {
      return true
    }

    addToast({
      title: translations.projectHome.startWritingBlockedTitle,
      description: manifest.writing_readiness.blockers.map(mapPlanningBlocker).join(' / '),
      variant: 'warning',
    })

    const targetRef = resolveRecommendedPlanningTarget(manifest)
    if (targetRef) {
      await openEditorTarget(targetRef, {
        revealLeftTree: true,
        switchLeftTab: true,
      })
    }

    return false
  }

  const refreshTree = createRefreshTree({ projectPath, setTree })
  const handleChapterSelect = createHandleChapterSelect({ projectPath, setCurrentChapter, setContent, setIsDirty })
  const rawHandleCreateVolume = createHandleCreateVolume({ projectPath, translations, setInputDialog, addToast, refreshTree })
  const rawHandleCreateChapter = createHandleCreateChapter({
    projectPath,
    treeLength: tree.length,
    translations,
    addToast,
    setInputDialog,
    setSelectVolumeDialog,
    onCreateVolume: rawHandleCreateVolume,
  })
  const rawHandleCreateChapterInVolume = createHandleCreateChapterInVolume({ projectPath, translations, addToast, refreshTree })
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
    onCreated: (relativePath, _title) => {
      setSelectedPath(`knowledge:${relativePath}`)
      void openEditorTarget(`knowledge:${relativePath}`, {
        revealLeftTree: true,
        switchLeftTab: true,
      })
    },
  })

  const handleCreateVolume = async () => {
    if (!await guardWritingEntry()) {
      return
    }
    await rawHandleCreateVolume()
  }

  const handleCreateChapter = async () => {
    if (!await guardWritingEntry()) {
      return
    }
    await rawHandleCreateChapter()
  }

  const handleCreateChapterInVolume = async (volumePath: string, chapterTitle: string) => {
    if (!await guardWritingEntry()) {
      return
    }
    await rawHandleCreateChapterInVolume(volumePath, chapterTitle)
  }

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
