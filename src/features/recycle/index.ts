import {
  emptyRecycleBin,
  emptyRecycledProjects,
  listRecycleItems,
  listRecycledProjects,
  permanentlyDeleteRecycleItem,
  permanentlyDeleteRecycledProject,
  restoreRecycleItem,
  restoreRecycledProject,
  type RecycleItem,
} from '@/lib/tauri-commands'

export type { RecycleItem }

export async function listRecycleItemsByProject(projectPath: string): Promise<RecycleItem[]> {
  return listRecycleItems(projectPath)
}

export async function listRecycledProjectsByRoot(rootDir: string): Promise<RecycleItem[]> {
  return listRecycledProjects(rootDir)
}

export async function restoreRecycleItemById(projectPath: string, itemId: string): Promise<void> {
  await restoreRecycleItem(projectPath, itemId)
}

export async function restoreRecycledProjectById(rootDir: string, itemId: string): Promise<void> {
  await restoreRecycledProject(rootDir, itemId)
}

export async function permanentlyDeleteRecycleItemById(projectPath: string, itemId: string): Promise<void> {
  await permanentlyDeleteRecycleItem(projectPath, itemId)
}

export async function permanentlyDeleteRecycledProjectById(rootDir: string, itemId: string): Promise<void> {
  await permanentlyDeleteRecycledProject(rootDir, itemId)
}

export async function emptyRecycleBinByProject(projectPath: string): Promise<void> {
  await emptyRecycleBin(projectPath)
}

export async function emptyRecycledProjectsByRoot(rootDir: string): Promise<void> {
  await emptyRecycledProjects(rootDir)
}
