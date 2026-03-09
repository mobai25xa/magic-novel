import { BookOpen } from 'lucide-react'

import { useProjectStore } from '@/stores/project-store'
import { useTranslation } from '@/hooks/use-translation'
import { useSettingsStore } from '@/state/settings'


import { useToast } from '@/magic-ui/components'

import { useHomePageStats } from './page/use-home-page-stats'
import { HeroCard } from './HeroCard'
import { StatCard } from './StatCard'
import { AiTipCard } from './AiTipCard'
import { BookShelf } from './BookShelf'
import { HomePageContextMenus } from './page/home-page-context-menus'
import { HomePageDialogs } from './page/home-page-dialogs'
import { createConfirmPendingAction } from './page/home-page-confirm-actions'
import { createHomeProjectActions } from './page/home-page-project-actions'
import { useHomePageController } from './page/use-home-page-controller'

interface WorkspacePageProps {
  onOpenProject: (path: string) => void
  onOpenCreate: () => void
  onOpenSettings?: () => void
}

export function WorkspacePage({ onOpenProject, onOpenCreate, onOpenSettings }: WorkspacePageProps) {
  const { translations, t } = useTranslation()
  const { addToast } = useToast()
  const projectsRootDir = useSettingsStore((state) => state.projectsRootDir)
  const { state, setters } = useHomePageController()
  const projectStore = useProjectStore()
  const stats = useHomePageStats(projectStore.projectList)

  const todayDeltaText = `${t('discover.today')} +${stats.todayWordDelta.toLocaleString()} ${t('discover.wordUnit')}`

  const projectActions = createHomeProjectActions({
    onOpenSettings: onOpenSettings ?? (() => {}),
    projectsRootDir,
    translations,
    addToast,
    projectStore,
    setters,
    getEditingProject: () => state.editingProject,
    getContextMenu: () => state.contextMenu,
    handleOpenProjectFolder: (projectPath?: string) => {
      const targetPath = projectPath ?? state.contextMenu?.projectPath
      if (!targetPath) return

      addToast({
        title: translations.home.hint,
        description: translations.home.openFolderInDev,
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

  return (
    <div className="workspace-page">
      <div className="bento-grid">
        <HeroCard onOpenProject={onOpenProject} />

        <StatCard
          icon={<BookOpen size={16} />}
          value={stats.totalWordCount.toLocaleString()}
          label={t('home.totalWords')}
          trend={{ direction: 'up', value: todayDeltaText }}
          iconTone="blue"
        />

        <AiTipCard />

        <BookShelf
          onOpenProject={onOpenProject}
          onCreateProject={onOpenCreate}
          onProjectContextMenu={projectActions.handleProjectContextMenu}
        />
      </div>

      <HomePageDialogs
        translations={translations}
        state={state}
        setters={setters}
        onCreateProject={projectActions.handleCreateProject}
        onEditProjectConfirm={projectActions.handleEditConfirm}
        onConfirmPendingAction={handleConfirmPendingAction}
        onImportProject={projectActions.handleProjectImport}
        onExportProject={projectActions.handleProjectExport}
      />

      <HomePageContextMenus
        contextMenu={state.contextMenu}
        recycledProjects={projectStore.recycledProjects}
        translations={translations}
        setContextMenu={setters.setContextMenu}
        onOpenProject={onOpenProject}
        onEditProject={projectActions.handleEditProject}
        onOpenProjectFolder={projectActions.handleOpenProjectFolder}
        onOpenImportDialog={(path) => {
          setters.setContextMenu(null)
          setters.setIoProjectPath(path)
          setters.setImportDialogOpen(true)
        }}
        onOpenExportDialog={(path) => {
          setters.setContextMenu(null)
          setters.setIoProjectPath(path)
          setters.setExportDialogOpen(true)
        }}
        onDeleteProject={projectActions.handleDeleteProjectPending}
        onRestoreProjectByPath={(id, path) => {
          void projectActions.handleRestoreProject(id, path)
        }}
        onPermanentDeleteByPath={projectActions.handlePermanentDeletePending}
      />
    </div>
  )
}
