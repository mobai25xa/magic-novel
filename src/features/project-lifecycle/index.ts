import {
  createProject,
  getProjectTree,
  importAsset,
  importManuscript,
  openProject,
  scanProjectsDirectory,
  updateProjectMetadata,
} from '@/lib/tauri-commands'
import { mapBackendTreeNode } from '@/features/shared/tree-node-mapper'

export type CreateProjectInput = {
  projectPath: string
  name: string
  author: string
}

export async function createProjectLifecycle(input: CreateProjectInput) {
  const snapshot = await createProject({
    path: input.projectPath,
    name: input.name,
    author: input.author,
  })
  return {
    snapshot,
    tree: snapshot.tree.map(mapBackendTreeNode),
  }
}

export async function openProjectLifecycle(path: string) {
  const snapshot = await openProject(path)
  return {
    snapshot,
    tree: snapshot.tree.map(mapBackendTreeNode),
  }
}

export async function refreshProjectTreeLifecycle(path: string) {
  const tree = await getProjectTree(path)
  return tree.map(mapBackendTreeNode)
}

export async function updateProjectMetadataLifecycle(
  path: string,
  name: string,
  author: string,
  description?: string,
) {
  return updateProjectMetadata(path, name, author, description)
}

export async function scanProjectsLifecycle(rootDir: string) {
  const snapshots = await scanProjectsDirectory(rootDir)
  return snapshots.map((snapshot) => ({
    ...snapshot,
    tree: snapshot.tree.map(mapBackendTreeNode),
  }))
}

export async function importManuscriptLifecycle(
  projectPath: string,
  inputPath: string,
) {
  await importManuscript(projectPath, inputPath)
}

export async function importAssetLifecycle(
  projectPath: string,
  inputPath: string,
  kind: string,
) {
  return importAsset(projectPath, inputPath, kind)
}
