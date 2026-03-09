import {
  moveChapterAndReloadProjectTree,
  renameProjectTreeNode,
} from '@/features/content-tree-management'
import type { useToast } from '@/magic-ui/components'
import type { Translations } from '@/i18n/locales/zh'

import { convertFileNode } from './content-tree-converters'
import type { FileNode } from './content-tree-types'

type AddToast = ReturnType<typeof useToast>['addToast']

type StoreSetters = {
  setTree: (tree: FileNode[]) => void
  setSelectedPath: (path: string | null) => void
}

export function createHandleSelect(input: {
  setSelectedPath: (path: string | null) => void
  onChapterSelect: (path: string, chapterId: string, title?: string) => void
  onAssetSelect?: (relativePath: string) => void
}) {
  return (node: FileNode) => {
    if (node.kind === 'chapter' && node.chapterId) {
      input.setSelectedPath(node.path)
      input.onChapterSelect(node.path, node.chapterId, node.title)
      return
    }

    if (node.kind === 'asset_file' && node.assetRelativePath) {
      input.setSelectedPath(node.path)
      input.onAssetSelect?.(node.assetRelativePath)
    }
  }
}

export function createHandleRename(input: {
  projectPath: string | null
  addToast: AddToast
  store: StoreSetters
  labels: Translations['tree']
}) {
  return async (node: FileNode, newName: string) => {
    if (!input.projectPath) {
      input.addToast({ title: input.labels.errorNoProject, description: input.labels.errorNoProject, variant: 'destructive' })
      return
    }

    if (node.kind === 'knowledge' || node.kind === 'asset_dir' || node.kind === 'asset_file') {
      input.addToast({ title: input.labels.renameNotSupported, description: input.labels.renameNotSupportedDesc, variant: 'info' })
      return
    }

    try {
      const nextTree = await renameProjectTreeNode(input.projectPath, {
        kind: node.kind,
        path: node.path,
        title: newName,
      })
      input.store.setTree(nextTree.map(convertFileNode))

      input.addToast({
        title: input.labels.renameSuccess,
        description: input.labels.renamedDesc.replace('{from}', node.title || node.name).replace('{to}', newName),
        variant: 'success',
      })
    } catch (error) {
      console.error('Failed to rename:', error)
      input.addToast({ title: input.labels.renameFailed, description: String(error), variant: 'destructive' })
    }
  }
}

export function createHandleMoveChapter(input: {
  projectPath: string | null
  selectedPath: string | null
  addToast: AddToast
  store: StoreSetters
  labels: Translations['tree']
}) {
  return async (chapterPath: string, targetVolumePath: string, targetIndex: number) => {
    if (!input.projectPath) {
      input.addToast({ title: input.labels.errorNoProject, description: input.labels.errorNoProject, variant: 'destructive' })
      return
    }

    try {
      const { newPath, tree } = await moveChapterAndReloadProjectTree(
        input.projectPath,
        chapterPath,
        targetVolumePath,
        targetIndex,
      )
      input.store.setTree(tree.map(convertFileNode))

      if (input.selectedPath === chapterPath) {
        input.store.setSelectedPath(newPath)
      }

      input.addToast({ title: input.labels.moveSuccess, description: input.labels.moveSuccessDesc, variant: 'success' })
    } catch (error) {
      console.error('Failed to move chapter:', error)
      input.addToast({ title: input.labels.moveFailed, description: String(error), variant: 'destructive' })
    }
  }
}
