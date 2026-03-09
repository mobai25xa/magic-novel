import { useProjectStore } from '@/state/project'
import { useSettingsStore } from '@/state/settings'
import { useTranslation } from '@/hooks/use-translation'

import { useToast } from '@/magic-ui/components'

import { createConfirmPendingAction } from './page/home-page-confirm-actions'
import { createHomeProjectActions } from './page/home-page-project-actions'
import { useHomePageController } from './page/use-home-page-controller'
import { useHomePageDerivedData } from './use-home-page-derived-data'
import { useHomePageHydration } from './page/use-home-page-hydration'
import { useHomePageStats } from './page/use-home-page-stats'

function useHomePageEnvironment() {
  const projectStore = useProjectStore()
  const { projectsRootDir, language } = useSettingsStore()
  const { addToast } = useToast()
  const { translations } = useTranslation()

  return { projectStore, projectsRootDir, language, addToast, translations }
}

type ProjectStoreState = ReturnType<typeof import('@/stores/project-store').useProjectStore.getState>

function useHomePageHydrationBridge(input: {
  projectsRootDir: string | null
  projectStore: ProjectStoreState
}) {
  useHomePageHydration({
    projectsRootDir: input.projectsRootDir,
    projectListLength: input.projectStore.projectList.length,
    recycledProjectsLength: input.projectStore.recycledProjects.length,
    replaceAllProjects: input.projectStore.replaceAllProjects,
    clearAllProjects: input.projectStore.clearAllProjects,
  })
}

export function useHomePageViewModel(onOpenSettings: () => void) {
  const { state, setters } = useHomePageController()
  const env = useHomePageEnvironment()

  useHomePageHydrationBridge({
    projectsRootDir: env.projectsRootDir,
    projectStore: env.projectStore,
  })

  const stats = useHomePageStats(env.projectStore.projectList)
  const derived = useHomePageDerivedData({
    typeFilter: state.typeFilter,
    projectList: env.projectStore.projectList,
    recycledProjects: env.projectStore.recycledProjects,
    genresByProjectPath: stats.genresByProjectPath,
  })

  const projectActions = createHomeProjectActions({
    onOpenSettings,
    projectsRootDir: env.projectsRootDir,
    translations: env.translations,
    addToast: env.addToast,
    projectStore: env.projectStore,
    setters,
    getEditingProject: () => state.editingProject,
    getContextMenu: () => state.contextMenu,
    handleOpenProjectFolder: (projectPath?: string) => {
      const targetPath = projectPath ?? state.contextMenu?.projectPath
      if (!targetPath) return
      env.addToast({
        title: env.translations.home.hint,
        description: env.translations.home.openFolderInDev,
        variant: 'info',
      })
      setters.setContextMenu(null)
    },
  })

  const handleConfirmPendingAction = createConfirmPendingAction({
    getPendingAction: () => state.pendingAction,
    getIsMutating: () => state.isMutating,
    setIsMutating: setters.setIsMutating,
    setConfirmDialog: setters.setConfirmDialog,
    setPendingAction: setters.setPendingAction,
    onMoveToRecycle: async (path: string) => {
      await projectActions.handleDeleteProject(path)
    },
    onPermanentDelete: async (id: string) => {
      await projectActions.handlePermanentDelete(id)
    },
  })

  return {
    state,
    setters,
    projectStore: env.projectStore,
    language: env.language,
    translations: env.translations,
    addToast: env.addToast,
    stats,
    derived,
    projectActions,
    handleConfirmPendingAction,
  }
}
