import {
  createKnowledgeDocument as createKnowledgeDocumentCommand,
  createKnowledgeFolder as createKnowledgeFolderCommand,
  deleteKnowledgeEntry as deleteKnowledgeEntryCommand,
  createAssetFile as createAssetFileCommand,
  createAssetFolder as createAssetFolderCommand,
  deleteAssetPath as deleteAssetPathCommand,
  getKnowledgeTree as getKnowledgeTreeCommand,
  trashChapter as trashChapterCommand,
  trashVolume as trashVolumeCommand,
  exportChapter as exportChapterCommand,
  exportVolume as exportVolumeCommand,
  getAssetsTree as getAssetsTreeCommand,
  getProjectTree as getProjectTreeCommand,
  importChapter as importChapterCommand,
  importManuscriptIntoVolume as importManuscriptIntoVolumeCommand,
  moveChapter as moveChapterCommand,
  updateChapterMetadata as updateChapterMetadataCommand,
  updateAssetFolderTitle as updateAssetFolderTitleCommand,
  updateAssetFileTitle as updateAssetFileTitleCommand,
  updateVolume as updateVolumeCommand,
  type AssetKind,
  type AssetLibraryNode,
  type FileNode,
  type KnowledgeTreeNode,
} from '@/lib/tauri-commands'

type ProjectTreeNodeKind = 'dir' | 'chapter'
type AssetTreeNodeKind = 'asset_dir' | 'asset_file'
type VolumeImportKind = 'manuscript' | 'chapter'

export type { AssetKind, AssetLibraryNode, FileNode, KnowledgeTreeNode }

export async function loadProjectTree(projectPath: string): Promise<FileNode[]> {
  return getProjectTreeCommand(projectPath)
}

export async function loadAssetsTree(projectPath: string): Promise<AssetLibraryNode[]> {
  return getAssetsTreeCommand(projectPath)
}

export async function loadKnowledgeTree(projectPath: string): Promise<KnowledgeTreeNode[]> {
  return getKnowledgeTreeCommand(projectPath)
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

export async function renameAssetNode(
  projectPath: string,
  input: { kind: AssetTreeNodeKind; relativePath: string; title: string },
): Promise<void> {
  if (input.kind === 'asset_file') {
    await updateAssetFileTitleCommand(projectPath, input.relativePath, input.title)
    return
  }

  await updateAssetFolderTitleCommand(projectPath, input.relativePath, input.title)
}

export async function createAssetFolderNode(
  projectPath: string,
  parentRelativeDir: string,
  title: string,
): Promise<string> {
  return createAssetFolderCommand(projectPath, parentRelativeDir, title)
}

export async function createAssetFileNode(
  projectPath: string,
  parentRelativeDir: string,
  assetKind: AssetKind,
  title: string,
): Promise<string> {
  return createAssetFileCommand(projectPath, parentRelativeDir, assetKind, title)
}

export async function deleteAssetNode(projectPath: string, relativePath: string): Promise<void> {
  await deleteAssetPathCommand(projectPath, relativePath)
}

export async function createKnowledgeFolderNode(
  projectPath: string,
  parentVirtualDir: string,
  name: string,
): Promise<string> {
  return createKnowledgeFolderCommand(projectPath, parentVirtualDir, name)
}

export async function createKnowledgeDocumentNode(
  projectPath: string,
  parentVirtualDir: string,
  name: string,
): Promise<string> {
  return createKnowledgeDocumentCommand(projectPath, parentVirtualDir, name)
}

export async function deleteKnowledgeNode(projectPath: string, virtualPath: string): Promise<void> {
  await deleteKnowledgeEntryCommand(projectPath, virtualPath)
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
  getKnowledgeTreeCommand as getKnowledgeTree,
  getAssetsTreeCommand as getAssetsTree,
  createKnowledgeFolderCommand as createKnowledgeFolder,
  createKnowledgeDocumentCommand as createKnowledgeDocument,
  createAssetFolderCommand as createAssetFolder,
  createAssetFileCommand as createAssetFile,
  deleteKnowledgeEntryCommand as deleteKnowledgeEntry,
  updateAssetFileTitleCommand as updateAssetFileTitle,
  updateAssetFolderTitleCommand as updateAssetFolderTitle,
  deleteAssetPathCommand as deleteAssetPath,
}
