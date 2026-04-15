import { open } from '@tauri-apps/plugin-dialog'

import { isTauriCommandUnavailableError } from '@/lib/tauri-command-errors'
import {
  buildCreateProjectCommandInput,
  buildStartProjectBootstrapInput,
} from '@/components/create/workflow-helpers'
import {
  createProjectEntry,
  exportProjectTree,
  importProjectAsset,
  importProjectManuscript,
  moveProjectToRecycle,
  openProjectEntry,
  refreshPlanningManifestEntry,
  removeRecycledProject,
  resumeProjectBootstrapEntry,
  restoreProjectFromRecycle,
  startProjectBootstrapEntry,
  updateProjectEntryMetadata,
} from '@/features/project-home'

import { convertFileNode, sanitizeFilename } from '../home-utils'

import type {
  HomeCreateProjectInput,
  HomeEditProject,
  HomeEditProjectInput,
  HomeImportKind,
} from './home-page-types'
import type {
  ProjectBootstrapStatus,
  ProjectSnapshot,
} from '@/features/project-home'

type ProjectStoreState = ReturnType<
  typeof import('@/stores/project-store').useProjectStore.getState
>

type ProjectStore = Pick<
  ProjectStoreState,
  'projectList' | 'setProjectPath' | 'setProject' | 'setTree' | 'setPlanningManifest' | 'addToProjectList'
>

export async function pickProjectPath(path?: string) {
  if (path) return path

  const result = await open({ directory: true, multiple: false, title: '' })
  if (!result || typeof result !== 'string') return null
  return result
}

async function openTextFile(title: string) {
  const selected = await open({
    title,
    multiple: false,
    directory: false,
    filters: [{ name: 'Text/Markdown', extensions: ['txt', 'md'] }],
  })
  if (!selected || typeof selected !== 'string') return null
  return selected
}

async function openOutputDir(title: string) {
  const outputDir = await open({ title, directory: true, multiple: false })
  if (!outputDir || typeof outputDir !== 'string') return null
  return outputDir
}

export async function runImport(input: { projectPath: string; kind: HomeImportKind; title: string }) {
  const selected = await openTextFile(input.title)
  if (!selected) return

  if (input.kind === 'manuscript') {
    await importProjectManuscript(input.projectPath, selected)
  } else {
    await importProjectAsset(input.projectPath, selected, input.kind)
  }
}

export async function runExport(input: {
  projectPath: string
  format: string
  title: string
  projectList: ProjectStore['projectList']
}) {
  const projectName = input.projectList.find((p) => p.path === input.projectPath)?.name || 'export'
  const safeName = sanitizeFilename(projectName)

  const outputDir = await openOutputDir(input.title)
  if (!outputDir) return

  await exportProjectTree(input.projectPath, `${outputDir}/${safeName}`, input.format)
}

export function syncCurrentProject(
  projectStore: Pick<ProjectStore, 'setProjectPath' | 'setProject'>,
  snapshot: Awaited<ReturnType<typeof openProjectEntry>>,
) {
  projectStore.setProjectPath(snapshot.path)
  projectStore.setProject({
    projectId: snapshot.project.project_id,
    name: snapshot.project.name,
    author: snapshot.project.author,
    description: snapshot.project.description,
    bootstrapState: snapshot.project.bootstrap_state,
    bootstrapUpdatedAt: snapshot.project.bootstrap_updated_at,
    createdAt: snapshot.project.created_at,
    updatedAt: snapshot.project.updated_at,
  })
}

type AddToast = ReturnType<typeof import('@/magic-ui/components').useToast>['addToast']

export type CreateProjectFlowResult = {
  snapshot: ProjectSnapshot
  bootstrapStatus: ProjectBootstrapStatus | null
  bootstrapError: string | null
  bootstrapUnsupported: boolean
}

function getErrorMessage(error: unknown) {
  if (error instanceof Error && error.message) {
    return error.message
  }

  return String(error)
}

function isBootstrapUnsupportedError(error: unknown) {
  return isTauriCommandUnavailableError(error, 'start_project_bootstrap')
}

export async function createProjectFlow(input: {
  onOpenSettings: () => void
  projectsRootDir: string | null
  projectStore: ProjectStore
  addToast: AddToast
  translations: { common: { error: string }; home: { configureRootDir: string; createSuccess: string; projectCreatedMsg: string } }
  data: HomeCreateProjectInput
  onBootstrapStatus?: (status: ProjectBootstrapStatus) => void
  suppressSuccessToast?: boolean
}): Promise<CreateProjectFlowResult> {
  if (!input.projectsRootDir) {
    input.addToast({
      title: input.translations.common.error,
      description: input.translations.home.configureRootDir,
      variant: 'destructive',
    })
    input.onOpenSettings()
    throw new Error(input.translations.home.configureRootDir)
  }

  const requestedPath = `${input.projectsRootDir}/${input.data.name}`
  const snapshot = await createProjectEntry(buildCreateProjectCommandInput(requestedPath, input.data))

  syncCurrentProject(input.projectStore, snapshot)
  input.projectStore.setTree([])
  input.projectStore.addToProjectList({
    path: snapshot.path,
    name: snapshot.project.name,
    author: snapshot.project.author,
    lastOpenedAt: Date.now(),
    coverImage: snapshot.project.cover_image,
  })

  if (!input.suppressSuccessToast) {
    input.addToast({
      title: input.translations.home.createSuccess,
      description: `${input.translations.home.projectCreatedMsg}${input.data.name}`,
      variant: 'success',
    })
  }

  if (!input.data.aiAssist) {
    return {
      snapshot,
      bootstrapStatus: null,
      bootstrapError: null,
      bootstrapUnsupported: false,
    }
  }

  try {
    const initialStatus = await startProjectBootstrapEntry(
      buildStartProjectBootstrapInput(snapshot.path, input.data),
    )
    input.onBootstrapStatus?.(initialStatus)

    return {
      snapshot,
      bootstrapStatus: initialStatus,
      bootstrapError: null,
      bootstrapUnsupported: false,
    }
  } catch (error) {
    return {
      snapshot,
      bootstrapStatus: null,
      bootstrapError: getErrorMessage(error),
      bootstrapUnsupported: isBootstrapUnsupportedError(error),
    }
  }
}

export async function resumeProjectBootstrapFlow(input: {
  projectPath: string
  onBootstrapStatus?: (status: ProjectBootstrapStatus) => void
}): Promise<ProjectBootstrapStatus> {
  const initialStatus = await resumeProjectBootstrapEntry(input.projectPath)
  input.onBootstrapStatus?.(initialStatus)
  return initialStatus
}

export async function openProjectFlow(input: {
  projectStore: ProjectStore
  selectedPath: string
}) {
  const snapshot = await openProjectEntry(input.selectedPath)
  const planningManifest = await refreshPlanningManifestEntry(snapshot.path)

  syncCurrentProject(input.projectStore, snapshot)
  input.projectStore.setTree(snapshot.tree.map(convertFileNode))
  input.projectStore.setPlanningManifest(snapshot.path, planningManifest)
  input.projectStore.addToProjectList({
    path: snapshot.path,
    name: snapshot.project.name,
    author: snapshot.project.author,
    lastOpenedAt: Date.now(),
    coverImage: snapshot.project.cover_image,
  })
}

export async function loadProjectForEdit(input: { selectedPath: string }): Promise<HomeEditProject> {
  const snapshot = await openProjectEntry(input.selectedPath)
  return {
    path: snapshot.path,
    name: snapshot.project.name,
    author: snapshot.project.author,
    description: snapshot.project.description,
    coverImage: snapshot.project.cover_image,
    projectType: snapshot.project.project_type,
  }
}

export async function updateProjectFlow(input: {
  projectStore: ProjectStore
  projectPath: string
  data: HomeEditProjectInput
}) {
  const metadata = await updateProjectEntryMetadata(
    input.projectPath,
    input.data.name,
    input.data.author,
    input.data.description,
    input.data.projectType,
    input.data.coverImage,
  )

  input.projectStore.addToProjectList({
    path: input.projectPath,
    name: metadata.name,
    author: metadata.author,
    lastOpenedAt: Date.now(),
    coverImage: metadata.cover_image,
  })
}

export async function trashProjectFlow(input: {
  projectPath: string
  onLocalRemove: () => void
}) {
  await moveProjectToRecycle(input.projectPath)
  input.onLocalRemove()
}

export async function restoreRecycledProjectFlow(input: {
  rootDir: string
  itemId: string
  onLocalRestore: () => void
}) {
  await restoreProjectFromRecycle(input.rootDir, input.itemId)
  input.onLocalRestore()
}

export async function permanentlyDeleteRecycledProjectFlow(input: {
  rootDir: string
  itemId: string
  onLocalRemove: () => void
}) {
  await removeRecycledProject(input.rootDir, input.itemId)
  input.onLocalRemove()
}
