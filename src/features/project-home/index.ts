import {
  createProject as createProjectCommand,
  emptyRecycledProjects as emptyRecycledProjectsCommand,
  exportTreeMulti as exportTreeMultiCommand,
  getProjectTree as getProjectTreeCommand,
  importAsset as importAssetCommand,
  importManuscript as importManuscriptCommand,
  listRecycledProjects as listRecycledProjectsCommand,
  openProject as openProjectCommand,
  permanentlyDeleteRecycledProject as permanentlyDeleteRecycledProjectCommand,
  restoreRecycledProject as restoreRecycledProjectCommand,
  scanProjectsDirectory as scanProjectsDirectoryCommand,
  trashProject as trashProjectCommand,
  updateProjectMetadata as updateProjectMetadataCommand,
  type FileNode,
  type ProjectMetadata,
  type ProjectSnapshot,
  type RecycleItem,
} from '@/lib/tauri-commands'

export type { FileNode, ProjectSnapshot }

export async function createProjectEntry(
  projectPath: string,
  name: string,
  author: string,
  projectType?: string[],
  coverImage?: string,
): Promise<ProjectSnapshot> {
  return createProjectCommand(projectPath, name, author, projectType, coverImage)
}

export async function openProjectEntry(projectPath: string): Promise<ProjectSnapshot> {
  return openProjectCommand(projectPath)
}

export async function loadProjectTree(projectPath: string): Promise<FileNode[]> {
  return getProjectTreeCommand(projectPath)
}

export async function updateProjectEntryMetadata(
  projectPath: string,
  name?: string,
  author?: string,
  description?: string,
  projectType?: string[],
  coverImage?: string,
): Promise<ProjectMetadata> {
  return updateProjectMetadataCommand(projectPath, name, author, description, projectType, coverImage)
}

export async function importProjectManuscript(projectPath: string, inputPath: string): Promise<void> {
  await importManuscriptCommand(projectPath, inputPath)
}

export async function importProjectAsset(projectPath: string, inputPath: string, kind: string): Promise<string> {
  return importAssetCommand(projectPath, inputPath, kind)
}

export async function exportProjectTree(
  projectPath: string,
  outputDir: string,
  format: string,
): Promise<number> {
  return exportTreeMultiCommand(projectPath, outputDir, format)
}

export async function moveProjectToRecycle(projectPath: string): Promise<void> {
  await trashProjectCommand(projectPath)
}

export async function listProjectRecycle(rootDir: string): Promise<RecycleItem[]> {
  return listRecycledProjectsCommand(rootDir)
}

export async function restoreProjectFromRecycle(rootDir: string, itemId: string): Promise<void> {
  await restoreRecycledProjectCommand(rootDir, itemId)
}

export async function removeRecycledProject(rootDir: string, itemId: string): Promise<void> {
  await permanentlyDeleteRecycledProjectCommand(rootDir, itemId)
}

export async function clearProjectRecycle(rootDir: string): Promise<void> {
  await emptyRecycledProjectsCommand(rootDir)
}

export async function scanProjects(rootDir: string): Promise<ProjectSnapshot[]> {
  return scanProjectsDirectoryCommand(rootDir)
}

export {
  createProjectCommand as createProject,
  openProjectCommand as openProject,
  updateProjectMetadataCommand as updateProjectMetadata,
  getProjectTreeCommand as getProjectTree,
  importManuscriptCommand as importManuscript,
  importAssetCommand as importAsset,
  exportTreeMultiCommand as exportTreeMulti,
  trashProjectCommand as trashProject,
  listRecycledProjectsCommand as listRecycledProjects,
  restoreRecycledProjectCommand as restoreRecycledProject,
  permanentlyDeleteRecycledProjectCommand as permanentlyDeleteRecycledProject,
  emptyRecycledProjectsCommand as emptyRecycledProjects,
  scanProjectsDirectoryCommand as scanProjectsDirectory,
}
