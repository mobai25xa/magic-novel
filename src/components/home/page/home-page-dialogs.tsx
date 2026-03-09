import { ConfirmDialog } from '@/components/common/ConfirmDialog'
import { SelectDialog } from '@/components/common/SelectDialog'

import { CreateProjectDialog } from '../CreateProjectDialog'
import { EditProjectDialog } from '../EditProjectDialog'

import type {
  HomeConfirmDialog,
  HomeControllerSetters,
  HomeControllerState,
  HomeEditProject,
  HomeEditProjectInput,
  HomeImportKind,
  HomePendingAction,
} from './home-page-types'

type Translations = ReturnType<typeof import('@/hooks/use-translation').useTranslation>['translations']

type DialogState = Pick<
  HomeControllerState,
  | 'dialogOpen'
  | 'editDialogOpen'
  | 'editingProject'
  | 'confirmDialog'
  | 'pendingAction'
  | 'isMutating'
  | 'ioProjectPath'
  | 'importDialogOpen'
  | 'exportDialogOpen'
>

type DialogSetters = Pick<
  HomeControllerSetters,
  | 'setDialogOpen'
  | 'setEditDialogOpen'
  | 'setEditingProject'
  | 'setConfirmDialog'
  | 'setPendingAction'
  | 'setIoProjectPath'
  | 'setImportDialogOpen'
  | 'setExportDialogOpen'
>

type CreateProjectDialogInput = {
  name: string
  author: string
  tags: string
  coverImage?: string
  projectType: string[]
}

type Input = {
  translations: Translations
  state: DialogState
  setters: DialogSetters
  onCreateProject: (data: CreateProjectDialogInput) => void
  onEditProjectConfirm: (data: HomeEditProjectInput) => void
  onConfirmPendingAction: () => void | Promise<void>
  onImportProject: (projectPath: string, kind: HomeImportKind) => void
  onExportProject: (projectPath: string, format: string) => void
}

function getImportOptions(pd: Translations['projectDialog']) {
  return [
    { value: 'manuscript', label: pd.importTypeManuscript },
    { value: 'worldview', label: pd.importTypeWorldview },
    { value: 'outline', label: pd.importTypeOutline },
    { value: 'character', label: pd.importTypeCharacter },
    { value: 'prompt', label: pd.importTypePrompt },
    { value: 'lore', label: pd.importTypeLore },
  ]
}

function getExportOptions() {
  return [
    { value: 'md', label: 'Markdown (.md)' },
    { value: 'txt', label: 'Text (.txt)' },
  ]
}

function closeConfirmDialog(input: {
  isMutating: boolean
  setConfirmDialog: (value: HomeConfirmDialog | null) => void
  setPendingAction: (value: HomePendingAction | null) => void
}) {
  if (input.isMutating) return
  input.setConfirmDialog(null)
  input.setPendingAction(null)
}

function closeIoDialogs(input: {
  setImportDialogOpen: (value: boolean) => void
  setExportDialogOpen: (value: boolean) => void
  setIoProjectPath: (value: string | null) => void
}) {
  input.setImportDialogOpen(false)
  input.setExportDialogOpen(false)
  input.setIoProjectPath(null)
}

function projectInitialData(project: HomeEditProject) {
  return {
    name: project.name,
    author: project.author,
    description: project.description,
    coverImage: project.coverImage,
    projectType: project.projectType,
  }
}

function renderEditProjectDialog(input: Input) {
  if (!input.state.editingProject) return null

  return (
    <EditProjectDialog
      open={input.state.editDialogOpen}
      initialData={projectInitialData(input.state.editingProject)}
      onClose={() => {
        input.setters.setEditDialogOpen(false)
        input.setters.setEditingProject(null)
      }}
      onConfirm={input.onEditProjectConfirm}
    />
  )
}

function renderConfirmDialog(input: Input) {
  if (!input.state.confirmDialog) return null

  return (
    <ConfirmDialog
      open={input.state.confirmDialog.open}
      title={input.state.confirmDialog.title}
      description={input.state.confirmDialog.description}
      danger={input.state.pendingAction?.type === 'permanent_delete'}
      onConfirm={() => {
        void input.onConfirmPendingAction()
      }}
      onCancel={() =>
        closeConfirmDialog({
          isMutating: input.state.isMutating,
          setConfirmDialog: input.setters.setConfirmDialog,
          setPendingAction: input.setters.setPendingAction,
        })
      }
    />
  )
}

function renderImportDialog(input: Input) {
  const close = () =>
    closeIoDialogs({
      setImportDialogOpen: input.setters.setImportDialogOpen,
      setExportDialogOpen: input.setters.setExportDialogOpen,
      setIoProjectPath: input.setters.setIoProjectPath,
    })

  return (
    <SelectDialog
      open={input.state.importDialogOpen}
      title={input.translations.editor.import}
      label={input.translations.projectDialog.importAs}
      options={getImportOptions(input.translations.projectDialog)}
      defaultValue="manuscript"
      onClose={close}
      onConfirm={(kind) => {
        if (input.state.ioProjectPath) {
          input.onImportProject(input.state.ioProjectPath, kind as HomeImportKind)
        }
        close()
      }}
    />
  )
}

function renderExportDialog(input: Input) {
  const close = () =>
    closeIoDialogs({
      setImportDialogOpen: input.setters.setImportDialogOpen,
      setExportDialogOpen: input.setters.setExportDialogOpen,
      setIoProjectPath: input.setters.setIoProjectPath,
    })

  return (
    <SelectDialog
      open={input.state.exportDialogOpen}
      title={input.translations.editor.export}
      label={input.translations.projectDialog.format}
      options={getExportOptions()}
      defaultValue="md"
      onClose={close}
      onConfirm={(format) => {
        if (input.state.ioProjectPath) {
          input.onExportProject(input.state.ioProjectPath, format)
        }
        close()
      }}
    />
  )
}

export function HomePageDialogs(input: Input) {
  return (
    <>
      <CreateProjectDialog
        open={input.state.dialogOpen}
        onClose={() => input.setters.setDialogOpen(false)}
        onConfirm={input.onCreateProject}
      />
      {renderEditProjectDialog(input)}
      {renderConfirmDialog(input)}
      {renderImportDialog(input)}
      {renderExportDialog(input)}
    </>
  )
}
