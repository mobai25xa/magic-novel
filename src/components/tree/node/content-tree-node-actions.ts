import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'

import {
  createKnowledgeDocumentNode,
  createKnowledgeFolderNode,
  deleteKnowledgeNode,
  createAssetFileNode,
  createAssetFolderNode,
  deleteAssetNode,
  exportProjectTreeNode,
  importVolumeNodeContent,
  loadProjectTree,
  renameAssetNode,
} from '@/features/content-tree-management'
import type { useToast } from '@/magic-ui/components'
import type { Translations } from '@/i18n/locales/zh'

import { convertFileNode } from '../content-tree-converters'
import type { TreeNodeProps } from '../content-tree-types'
import { sanitizeFilename } from '../content-tree-utils'

type Input = {
  node: TreeNodeProps['node']
  onSelect: TreeNodeProps['onSelect']
  onDelete: TreeNodeProps['onDelete']
  onRename: TreeNodeProps['onRename']
  projectPath: string | null
  setTree: (tree: TreeNodeProps['node'][]) => void
  addToast: ReturnType<typeof useToast>['addToast']
  currentAssetPath: string | null
  setCurrentAsset: (relativePath: string | null, title?: string | null) => void
  labels: Translations['tree']
}

function isKnowledgeVirtualPath(path?: string | null) {
  return typeof path === 'string' && path.startsWith('.magic_novel/')
}

async function refreshTree(input: Input) {
  if (!input.projectPath) return

  try {
    const newTree = await loadProjectTree(input.projectPath)
    input.setTree(newTree.map(convertFileNode))
  } catch (error) {
    console.error('Failed to refresh tree:', error)
  }
}

async function deleteAssetIfNeeded(input: Input): Promise<boolean> {
  if (!input.projectPath || !input.node.assetRelativePath) return false
  if (input.node.kind !== 'asset_file' && input.node.kind !== 'asset_dir') return false

  try {
    if (isKnowledgeVirtualPath(input.node.assetRelativePath)) {
      await deleteKnowledgeNode(input.projectPath, input.node.assetRelativePath)
    } else {
      await deleteAssetNode(input.projectPath, input.node.assetRelativePath)
    }

    const isCurrentAsset =
      input.node.kind === 'asset_file' &&
      !!input.currentAssetPath &&
      input.currentAssetPath === input.node.assetRelativePath
    if (isCurrentAsset) {
      input.setCurrentAsset(null)
    }

    await refreshTree(input)
    input.addToast({
      title: input.labels.deleteSuccess,
      description: `已删除 "${input.node.title || input.node.name}"`,
      variant: 'success',
    })
    return true
  } catch (error) {
    console.error('Failed to delete asset path:', error)
    input.addToast({ title: input.labels.deleteFailed, description: String(error), variant: 'destructive' })
    return true
  }
}

async function importTextFile(input: { title: string }) {
  const selected = await openDialog({
    title: input.title,
    multiple: false,
    directory: false,
    filters: [{ name: 'Text/Markdown', extensions: ['txt', 'md'] }],
  })

  if (!selected || typeof selected !== 'string') {
    return null
  }

  return selected
}

async function handleImportManuscript(input: Input) {
  if (!input.projectPath || input.node.kind !== 'dir') return

  try {
    const selected = await importTextFile({ title: input.labels.importDialogTitle })
    if (!selected) return

    await importVolumeNodeContent(input.projectPath, input.node.path, selected, 'manuscript')
    await refreshTree(input)
    input.addToast({ title: input.labels.importSuccess, description: input.labels.importToVolumeSuccess, variant: 'success' })
  } catch (error) {
    console.error('Failed to import manuscript into volume:', error)
    input.addToast({ title: input.labels.importFailed, description: String(error), variant: 'destructive' })
  }
}

async function handleImportChapter(input: Input) {
  if (!input.projectPath || input.node.kind !== 'dir') return

  try {
    const selected = await importTextFile({ title: input.labels.importDialogTitle })
    if (!selected) return

    await importVolumeNodeContent(input.projectPath, input.node.path, selected, 'chapter')
    await refreshTree(input)
    input.addToast({ title: input.labels.importSuccess, description: input.labels.importChapterSuccess, variant: 'success' })
  } catch (error) {
    console.error('Failed to import chapter:', error)
    input.addToast({ title: input.labels.importFailed, description: String(error), variant: 'destructive' })
  }
}

async function handleExportNode(input: Input, format: string) {
  if (!input.projectPath) return

  try {
    const defaultName = sanitizeFilename(input.node.title || input.node.name || 'export')
    const outputPath = await saveDialog({
      title: input.labels.exportDialogTitle,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
      defaultPath: `${defaultName}.${format}`,
    })
    if (!outputPath || typeof outputPath !== 'string') return

    if (input.node.kind !== 'chapter' && input.node.kind !== 'dir') {
      return
    }

    await exportProjectTreeNode(input.projectPath, {
      kind: input.node.kind,
      path: input.node.path,
      outputPath,
      format,
    })

    input.addToast({ title: input.labels.exportSuccess, description: outputPath, variant: 'success' })
  } catch (error) {
    console.error('Failed to export:', error)
    input.addToast({ title: input.labels.exportFailed, description: String(error), variant: 'destructive' })
  }
}

async function handleRenameNode(input: Input, newName: string) {
  if (!input.projectPath) return

  if (input.node.kind === 'asset_file' && input.node.assetRelativePath) {
    try {
      await renameAssetNode(input.projectPath, {
        kind: 'asset_file',
        relativePath: input.node.assetRelativePath,
        title: newName,
      })
      await refreshTree(input)
      input.addToast({ title: input.labels.renameSuccess, description: input.labels.renameFileSuccess, variant: 'success' })
    } catch (error) {
      console.error('Failed to rename asset file:', error)
      input.addToast({ title: input.labels.renameFailed, description: String(error), variant: 'destructive' })
    }
    return
  }

  if (input.node.kind === 'asset_dir' && input.node.assetRelativePath) {
    try {
      await renameAssetNode(input.projectPath, {
        kind: 'asset_dir',
        relativePath: input.node.assetRelativePath,
        title: newName,
      })
      await refreshTree(input)
      input.addToast({ title: input.labels.renameSuccess, description: input.labels.renameFolderSuccess, variant: 'success' })
    } catch (error) {
      console.error('Failed to rename asset folder:', error)
      input.addToast({ title: input.labels.renameFailed, description: String(error), variant: 'destructive' })
    }
    return
  }

  await input.onRename(input.node, newName)
}

async function handleCreateFolder(input: Input, title: string) {
  if (!input.projectPath) return

  try {
    const parentDir = input.node.kind === 'knowledge' ? '' : input.node.assetRelativePath || ''
    if (!parentDir || isKnowledgeVirtualPath(parentDir)) {
      await createKnowledgeFolderNode(input.projectPath, parentDir, title)
    } else {
      await createAssetFolderNode(input.projectPath, parentDir, title)
    }
    await refreshTree(input)
    input.addToast({ title: input.labels.createSuccess, description: input.labels.createFolderSuccess, variant: 'success' })
  } catch (error) {
    console.error('Failed to create folder:', error)
    input.addToast({ title: input.labels.createFailed, description: String(error), variant: 'destructive' })
  }
}

async function handleCreateFile(input: Input, title: string) {
  if (!input.projectPath) return

  try {
    const parentDir = input.node.kind === 'knowledge' ? '' : input.node.assetRelativePath || ''
    const isKnowledgeTarget = !parentDir || isKnowledgeVirtualPath(parentDir)
    const relativePath = isKnowledgeTarget
      ? await createKnowledgeDocumentNode(input.projectPath, parentDir, title)
      : await createAssetFileNode(input.projectPath, parentDir, 'worldview', title)

    await refreshTree(input)

    try {
      const normalizedTitle = title.replace(/\.md$/i, '')
      input.onSelect({
        kind: 'asset_file',
        name: relativePath.split('/').pop() || relativePath,
        title: normalizedTitle,
        path: isKnowledgeTarget ? `knowledge:${relativePath}` : `assets/${relativePath}`,
        assetRelativePath: relativePath,
      })
    } catch {}

    input.addToast({ title: input.labels.createSuccess, description: input.labels.createFileSuccess, variant: 'success' })
  } catch (error) {
    console.error('Failed to create file:', error)
    input.addToast({ title: input.labels.createFailed, description: String(error), variant: 'destructive' })
  }
}

export function createTreeNodeActions(input: Input) {
  return {
    handleDelete: async () => {
      const deleted = await deleteAssetIfNeeded(input)
      if (!deleted) {
        input.onDelete(input.node)
      }
    },
    handleImportManuscriptHere: async () => handleImportManuscript(input),
    handleImportChapterHere: async () => handleImportChapter(input),
    handleExport: async (format: string) => handleExportNode(input, format),
    handleRenameConfirm: async (newName: string) => handleRenameNode(input, newName),
    handleCreateFolder: async (title: string) => handleCreateFolder(input, title),
    handleCreateFile: async (title: string) => handleCreateFile(input, title),
  }
}
