import type {
  HomeContextMenu,
  HomeCreateProjectInput,
  HomeEditProject,
  HomeEditProjectInput,
  HomeImportKind,
} from './home-page-types'
import {
  createProjectFlow,
  loadProjectForEdit,
  openProjectFlow,
  permanentlyDeleteRecycledProjectFlow,
  pickProjectPath,
  restoreRecycledProjectFlow,
  runExport,
  runImport,
  trashProjectFlow,
  updateProjectFlow,
} from './home-page-project-actions-helpers'
import { eventBus, EVENTS } from '@/lib/events'

type AddToast = ReturnType<typeof import('@/magic-ui/components').useToast>['addToast']
type Translations = ReturnType<typeof import('@/hooks/use-translation').useTranslation>['translations']

type ProjectStoreState = ReturnType<
  typeof import('@/stores/project-store').useProjectStore.getState
>

type ProjectStore = Pick<
  ProjectStoreState,
  | 'projectList'
  | 'setProjectPath'
  | 'setProject'
  | 'setTree'
  | 'addToProjectList'
  | 'removeProjectFromList'
  | 'removeRecycledProjectById'
>

type Setters = {
  setEditingProject: (value: HomeEditProject | null) => void
  setEditDialogOpen: (value: boolean) => void
  setContextMenu: (value: HomeContextMenu | null) => void
  setPendingAction: (value: import('./home-page-types').HomePendingAction | null) => void
  setConfirmDialog: (value: { open: boolean; title: string; description: string } | null) => void
}

type Input = {
  onOpenSettings: () => void
  projectsRootDir: string | null
  translations: Translations
  addToast: AddToast
  projectStore: ProjectStore
  setters: Setters
  getEditingProject: () => HomeEditProject | null
  getContextMenu: () => HomeContextMenu | null
  handleOpenProjectFolder: (projectPath?: string) => void
}

function toastError(addToast: AddToast, title: string, error: unknown) {
  addToast({ title, description: String(error), variant: 'destructive' })
}

function makeDeleteProjectHandler(input: Input) {
  return async (path: string, event?: React.MouseEvent) => {
    event?.preventDefault()
    event?.stopPropagation()

    try {
      await trashProjectFlow({
        projectPath: path,
        onLocalRemove: () => input.projectStore.removeProjectFromList(path),
      })
      eventBus.emit(EVENTS.RECYCLE_REFRESH_REQUESTED)
      input.addToast({
        title: input.translations.home.deleteProjectTitle,
        description: input.translations.home.deleteProjectDesc,
        variant: 'success',
      })
    } catch (error) {
      toastError(input.addToast, input.translations.common.error, error)
    }
  }
}

function makeProjectContextMenuHandler(input: Input) {
  return (path: string, event: React.MouseEvent) => {
    event.preventDefault()
    event.stopPropagation()
    input.setters.setContextMenu({ x: event.clientX, y: event.clientY, projectPath: path })
  }
}

function makeEditProjectHandler(input: Input) {
  return async (projectPath?: string) => {
    const targetPath = projectPath ?? input.getContextMenu()?.projectPath
    if (!targetPath) return

    try {
      input.setters.setEditingProject(await loadProjectForEdit({ selectedPath: targetPath }))
      input.setters.setEditDialogOpen(true)
    } catch (error) {
      console.error('Failed to load project for edit:', error)
      toastError(input.addToast, input.translations.common.error, error)
    } finally {
      input.setters.setContextMenu(null)
    }
  }
}

function makeEditConfirmHandler(input: Input) {
  return async (data: HomeEditProjectInput) => {
    const editingProject = input.getEditingProject()
    if (!editingProject) return

    try {
      await updateProjectFlow({ projectStore: input.projectStore, projectPath: editingProject.path, data })
      input.addToast({
        title: input.translations.home.editSuccess,
        description: `${input.translations.home.projectUpdatedMsg}${data.name}`,
        variant: 'success',
      })
    } catch (error) {
      console.error('Failed to update project:', error)
      toastError(input.addToast, input.translations.home.editFailed, error)
    }
  }
}

function makeProjectImportHandler(input: Input) {
  return async (projectPath: string, kind: HomeImportKind) => {
    try {
      await runImport({ projectPath, kind, title: input.translations.editor.import })
      input.addToast({
        title: input.translations.common.success,
        description: input.translations.editor.import,
        variant: 'success',
      })
    } catch (error) {
      console.error('Failed to import:', error)
      toastError(input.addToast, input.translations.common.error, error)
    }
  }
}

function makeProjectExportHandler(input: Input) {
  return async (projectPath: string, format: string) => {
    try {
      await runExport({
        projectPath,
        format,
        title: input.translations.editor.export,
        projectList: input.projectStore.projectList,
      })
      input.addToast({
        title: input.translations.common.success,
        description: input.translations.editor.export,
        variant: 'success',
      })
    } catch (error) {
      console.error('Failed to export:', error)
      toastError(input.addToast, input.translations.common.error, error)
    }
  }
}

function makeCreateProjectHandler(input: Input) {
  return async (data: HomeCreateProjectInput) => {
    try {
      await createProjectFlow({
        onOpenSettings: input.onOpenSettings,
        projectsRootDir: input.projectsRootDir,
        projectStore: input.projectStore,
        addToast: input.addToast,
        translations: {
          common: { error: input.translations.common.error },
          home: {
            configureRootDir: input.translations.home.configureRootDir,
            createSuccess: input.translations.home.createSuccess,
            projectCreatedMsg: input.translations.home.projectCreatedMsg,
          },
        },
        data,
      })
    } catch (error) {
      console.error('Failed to create project:', error)
      toastError(input.addToast, input.translations.home.createFailed, error)
    }
  }
}

function makeOpenProjectHandler(input: Input) {
  return async (path?: string) => {
    try {
      const selectedPath = await pickProjectPath(path)
      if (!selectedPath) return
      await openProjectFlow({ projectStore: input.projectStore, selectedPath })
    } catch (error) {
      console.error('Failed to open project:', error)
    }
  }
}

export function createHomeProjectActions(input: Input) {
  return {
    handleDeleteProject: makeDeleteProjectHandler(input),
    handleDeleteProjectPending: (path: string) => {
      input.setters.setPendingAction({ type: 'move_to_recycle', path })
      input.setters.setConfirmDialog({
        open: true,
        title: input.translations.home.deleteProjectTitle,
        description: input.translations.home.deleteProjectDesc,
      })
    },
    handleProjectContextMenu: makeProjectContextMenuHandler(input),
    handleEditProject: makeEditProjectHandler(input),
    handleEditConfirm: makeEditConfirmHandler(input),
    handleOpenProjectFolder: input.handleOpenProjectFolder,
    handleProjectImport: makeProjectImportHandler(input),
    handleProjectExport: makeProjectExportHandler(input),
    handleCreateProject: makeCreateProjectHandler(input),
    handleOpenProject: makeOpenProjectHandler(input),
    handleRestoreProject: async (id: string, path: string, event?: React.MouseEvent) => {
      event?.stopPropagation()
      if (!input.projectsRootDir) return

      try {
        await restoreRecycledProjectFlow({
          rootDir: input.projectsRootDir,
          itemId: id,
          onLocalRestore: () => {
            input.projectStore.removeRecycledProjectById(id)
            void openProjectFlow({ projectStore: input.projectStore, selectedPath: path })
          },
        })
        input.addToast({
          title: input.translations.home.restoreSuccess,
          description: input.translations.home.projectRestoredMsg,
          variant: 'success',
        })
      } catch (error) {
        toastError(input.addToast, input.translations.common.error, error)
        throw error
      }
    },
    handlePermanentDeletePending: (id: string) => {
      input.setters.setPendingAction({ type: 'permanent_delete', id })
      input.setters.setConfirmDialog({
        open: true,
        title: input.translations.home.permanentDeleteTitle,
        description: input.translations.home.permanentDeleteDesc,
      })
    },
    handlePermanentDelete: async (id: string) => {
      if (!input.projectsRootDir) return

      try {
        await permanentlyDeleteRecycledProjectFlow({
          rootDir: input.projectsRootDir,
          itemId: id,
          onLocalRemove: () => input.projectStore.removeRecycledProjectById(id),
        })
        input.addToast({
          title: input.translations.home.permanentDeleteTitle,
          description: input.translations.recyclePage.deleteSuccess,
          variant: 'success',
        })
      } catch (error) {
        toastError(input.addToast, input.translations.common.error, error)
        throw error
      }
    },
  }
}
