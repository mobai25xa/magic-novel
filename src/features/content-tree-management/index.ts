import {
  createMagicAssetFile as createMagicAssetFileCommand,
  createMagicAssetFolder as createMagicAssetFolderCommand,
  deleteMagicAssetPath as deleteMagicAssetPathCommand,
  trashChapter as trashChapterCommand,
  trashVolume as trashVolumeCommand,
  exportChapter as exportChapterCommand,
  exportVolume as exportVolumeCommand,
  getMagicAssetsTree as getMagicAssetsTreeCommand,
  getProjectTree as getProjectTreeCommand,
  importChapter as importChapterCommand,
  importManuscriptIntoVolume as importManuscriptIntoVolumeCommand,
  moveChapter as moveChapterCommand,
  updateChapterMetadata as updateChapterMetadataCommand,
  updateMagicAssetFolderTitle as updateMagicAssetFolderTitleCommand,
  updateMagicAssetTitle as updateMagicAssetTitleCommand,
  updateVolume as updateVolumeCommand,
  type AssetKind,
  type FileNode,
  type MagicAssetNode,
} from '@/lib/tauri-commands'

type ProjectTreeNodeKind = 'dir' | 'chapter'
type AssetTreeNodeKind = 'asset_dir' | 'asset_file'
type VolumeImportKind = 'manuscript' | 'chapter'

export type { AssetKind, FileNode, MagicAssetNode }

export async function loadProjectTree(projectPath: string): Promise<FileNode[]> {
  return getProjectTreeCommand(projectPath)
}

export async function loadMagicAssetsTree(projectPath: string): Promise<MagicAssetNode[]> {
  return getMagicAssetsTreeCommand(projectPath)
}

export async function renameProjectTreeNode(
  projectPath: string,
  input: { kind: ProjectTreeNodeKind; path: string; title: string },
): Promise<FileNode[]> {
  if (input.kind === 'dir') {
    await updateVolumeCommand(projectPath, input.path, input.title)
  } else {
    await updateChapterMetadataCommand(projectPath, input.path, { title: input.title })
  }

  return loadProjectTree(projectPath)
}

export async function deleteProjectTreeNode(
  projectPath: string,
  input: { kind: ProjectTreeNodeKind; path: string },
): Promise<FileNode[]> {
  if (input.kind === 'dir') {
    await trashVolumeCommand(projectPath, input.path)
  } else {
    await trashChapterCommand(projectPath, input.path)
  }

  return loadProjectTree(projectPath)
}

export async function moveChapterAndReloadProjectTree(
  projectPath: string,
  chapterPath: string,
  targetVolumePath: string,
  targetIndex: number,
): Promise<{ newPath: string; tree: FileNode[] }> {
  const newPath = await moveChapterCommand(projectPath, chapterPath, targetVolumePath, targetIndex)
  const tree = await loadProjectTree(projectPath)
  return { newPath, tree }
}

export async function importVolumeNodeContent(
  projectPath: string,
  volumePath: string,
  inputPath: string,
  kind: VolumeImportKind,
): Promise<void> {
  if (kind === 'manuscript') {
    await importManuscriptIntoVolumeCommand(projectPath, volumePath, inputPath)
    return
  }

  await importChapterCommand(projectPath, volumePath, inputPath)
}

export async function exportProjectTreeNode(
  projectPath: string,
  input: { kind: ProjectTreeNodeKind; path: string; outputPath: string; format: string },
): Promise<void> {
  if (input.kind === 'chapter') {
    await exportChapterCommand(projectPath, input.path, input.outputPath, input.format)
    return
  }

  await exportVolumeCommand(projectPath, input.path, input.outputPath, input.format)
}

export async function renameMagicAssetNode(
  projectPath: string,
  input: { kind: AssetTreeNodeKind; relativePath: string; title: string },
): Promise<void> {
  if (input.kind === 'asset_file') {
    await updateMagicAssetTitleCommand(projectPath, input.relativePath, input.title)
    return
  }

  await updateMagicAssetFolderTitleCommand(projectPath, input.relativePath, input.title)
}

export async function createMagicAssetFolderNode(
  projectPath: string,
  parentRelativeDir: string,
  title: string,
): Promise<string> {
  return createMagicAssetFolderCommand(projectPath, parentRelativeDir, title)
}

export async function createMagicAssetFileNode(
  projectPath: string,
  parentRelativeDir: string,
  assetKind: AssetKind,
  title: string,
): Promise<string> {
  return createMagicAssetFileCommand(projectPath, parentRelativeDir, assetKind, title)
}

export async function deleteMagicAssetNode(projectPath: string, relativePath: string): Promise<void> {
  await deleteMagicAssetPathCommand(projectPath, relativePath)
}

export {
  trashChapterCommand as trashChapter,
  trashVolumeCommand as trashVolume,
  getProjectTreeCommand as getProjectTree,
  updateVolumeCommand as updateVolume,
  updateChapterMetadataCommand as updateChapterMetadata,
  moveChapterCommand as moveChapter,
  importManuscriptIntoVolumeCommand as importManuscriptIntoVolume,
  importChapterCommand as importChapter,
  exportChapterCommand as exportChapter,
  exportVolumeCommand as exportVolume,
  getMagicAssetsTreeCommand as getMagicAssetsTree,
  createMagicAssetFolderCommand as createMagicAssetFolder,
  createMagicAssetFileCommand as createMagicAssetFile,
  updateMagicAssetTitleCommand as updateMagicAssetTitle,
  updateMagicAssetFolderTitleCommand as updateMagicAssetFolderTitle,
  deleteMagicAssetPathCommand as deleteMagicAssetPath,
}
