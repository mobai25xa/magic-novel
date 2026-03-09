import type {
  Chapter,
  FileNode as ProjectFileNode,
  ProjectMetadata,
  ProjectSnapshot,
  ProjectType,
  RecycleItem,
  VolumeMetadata,
} from '@/platform/tauri/clients/project-client'
import {
  clearWritingStatsClient,
  createChapterClient,
  createProjectClient,
  createVolumeClient,
  emptyRecycleBinClient,
  emptyRecycledProjectsClient,
  exportBookSingleClient,
  exportChapterClient,
  exportTreeMultiClient,
  exportVolumeClient,
  getProjectTreeClient,
  importAssetClient,
  importChapterClient,
  importManuscriptClient,
  importManuscriptIntoVolumeClient,
  listRecycleItemsClient,
  listRecycledProjectsClient,
  moveChapterClient,
  openProjectClient,
  permanentlyDeleteRecycleItemClient,
  permanentlyDeleteRecycledProjectClient,
  readChapterClient,
  readVolumeClient,
  restoreRecycleItemClient,
  restoreRecycledProjectClient,
  saveChapterClient,
  saveChapterMarkdownClient,
  scanProjectsDirectoryClient,
  setChapterWordGoalClient,
  trashChapterClient,
  trashProjectClient,
  trashVolumeClient,
  updateChapterMetadataClient,
  updateProjectMetadataClient,
  updateVolumeClient,
} from '@/platform/tauri/clients/project-client'

export async function createProject(
  path: string,
  name: string,
  author: string,
  projectType?: string[],
  coverImage?: string,
): Promise<ProjectSnapshot> {
  return createProjectClient(path, name, author, projectType, coverImage)
}

export async function openProject(path: string): Promise<ProjectSnapshot> {
  return openProjectClient(path)
}

export type FileNode = ProjectFileNode | {
  kind: 'knowledge' | 'asset_dir' | 'asset_file'
  name: string
  path: string
  children?: FileNode[]
  chapter_id?: string
  title?: string
  text_length_no_whitespace?: number
  word_count?: number
  status?: string
  updated_at?: number
  created_at?: number
  assetRelativePath?: string
}

export async function getProjectTree(path: string): Promise<FileNode[]> {
  return getProjectTreeClient(path) as unknown as FileNode[]
}

export async function updateProjectMetadata(
  path: string,
  name?: string,
  author?: string,
  description?: string,
  projectType?: string[],
  coverImage?: string,
): Promise<ProjectMetadata> {
  return updateProjectMetadataClient(path, name, author, description, projectType, coverImage)
}

export async function scanProjectsDirectory(rootDir: string): Promise<ProjectSnapshot[]> {
  return scanProjectsDirectoryClient(rootDir)
}

export async function trashProject(projectPath: string): Promise<void> {
  return trashProjectClient(projectPath)
}

export async function listRecycledProjects(rootDir: string): Promise<RecycleItem[]> {
  return listRecycledProjectsClient(rootDir)
}

export async function restoreRecycledProject(rootDir: string, itemId: string): Promise<void> {
  return restoreRecycledProjectClient(rootDir, itemId)
}

export async function permanentlyDeleteRecycledProject(rootDir: string, itemId: string): Promise<void> {
  return permanentlyDeleteRecycledProjectClient(rootDir, itemId)
}

export async function emptyRecycledProjects(rootDir: string): Promise<void> {
  return emptyRecycledProjectsClient(rootDir)
}

export async function clearWritingStats(rootDir?: string): Promise<void> {
  return clearWritingStatsClient(rootDir)
}

export async function createVolume(projectPath: string, title: string): Promise<VolumeMetadata> {
  return createVolumeClient(projectPath, title)
}

export async function readVolume(projectPath: string, volumePath: string): Promise<VolumeMetadata> {
  return readVolumeClient(projectPath, volumePath)
}

export async function updateVolume(
  projectPath: string,
  volumePath: string,
  title?: string,
  summary?: string,
): Promise<VolumeMetadata> {
  return updateVolumeClient(projectPath, volumePath, title, summary)
}

export async function trashVolume(projectPath: string, volumePath: string): Promise<void> {
  return trashVolumeClient(projectPath, volumePath)
}

export async function listRecycleItems(projectPath: string): Promise<RecycleItem[]> {
  return listRecycleItemsClient(projectPath)
}

export async function restoreRecycleItem(projectPath: string, itemId: string): Promise<void> {
  return restoreRecycleItemClient(projectPath, itemId)
}

export async function permanentlyDeleteRecycleItem(projectPath: string, itemId: string): Promise<void> {
  return permanentlyDeleteRecycleItemClient(projectPath, itemId)
}

export async function emptyRecycleBin(projectPath: string): Promise<void> {
  return emptyRecycleBinClient(projectPath)
}

export async function createChapter(projectPath: string, volumePath: string, title: string): Promise<Chapter> {
  return createChapterClient(projectPath, volumePath, title)
}

export async function readChapter(projectPath: string, chapterPath: string): Promise<Chapter> {
  return readChapterClient(projectPath, chapterPath)
}

export async function saveChapter(
  projectPath: string,
  chapterPath: string,
  content: unknown,
  title?: string,
): Promise<Chapter> {
  return saveChapterClient(projectPath, chapterPath, content, title)
}

export async function saveChapterMarkdown(
  projectPath: string,
  markdownPath: string,
  content: string,
): Promise<void> {
  return saveChapterMarkdownClient(projectPath, markdownPath, content)
}

export async function updateChapterMetadata(
  projectPath: string,
  chapterPath: string,
  options: {
    title?: string
    summary?: string
    status?: string
    targetWords?: number
    tags?: string[]
    pinnedAssets?: { kind: string; assetId: string; nodeIds?: string[] }[] | null
  },
): Promise<Chapter> {
  return updateChapterMetadataClient(projectPath, chapterPath, options)
}

export async function setChapterWordGoal(
  projectPath: string,
  chapterPath: string,
  wordGoal: number | null,
): Promise<Chapter> {
  return setChapterWordGoalClient(projectPath, chapterPath, wordGoal)
}

export async function trashChapter(projectPath: string, chapterPath: string): Promise<void> {
  return trashChapterClient(projectPath, chapterPath)
}

export async function moveChapter(
  projectPath: string,
  chapterPath: string,
  targetVolumePath: string,
  targetIndex: number,
): Promise<string> {
  return moveChapterClient(projectPath, chapterPath, targetVolumePath, targetIndex)
}

export async function importAsset(projectPath: string, inputPath: string, kind: string): Promise<string> {
  return importAssetClient(projectPath, inputPath, kind)
}

export async function importManuscript(projectPath: string, inputPath: string): Promise<void> {
  return importManuscriptClient(projectPath, inputPath)
}

export async function importManuscriptIntoVolume(projectPath: string, volumePath: string, inputPath: string): Promise<void> {
  return importManuscriptIntoVolumeClient(projectPath, volumePath, inputPath)
}

export async function importChapter(projectPath: string, volumePath: string, inputPath: string, title?: string): Promise<string> {
  return importChapterClient(projectPath, volumePath, inputPath, title)
}

export async function exportChapter(projectPath: string, chapterPath: string, outputPath: string, format: string): Promise<void> {
  return exportChapterClient(projectPath, chapterPath, outputPath, format)
}

export async function exportVolume(projectPath: string, volumePath: string, outputPath: string, format: string): Promise<void> {
  return exportVolumeClient(projectPath, volumePath, outputPath, format)
}

export async function exportBookSingle(projectPath: string, outputPath: string, format: string): Promise<void> {
  return exportBookSingleClient(projectPath, outputPath, format)
}

export async function exportTreeMulti(projectPath: string, outputDir: string, format: string): Promise<number> {
  return exportTreeMultiClient(projectPath, outputDir, format)
}

export type {
  Chapter,
  ProjectMetadata,
  ProjectSnapshot,
  ProjectType,
  RecycleItem,
  VolumeMetadata,
}

