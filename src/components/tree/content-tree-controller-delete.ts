import {
  deleteProjectTreeNode,
} from '@/features/content-tree-management'
import type { useToast } from '@/magic-ui/components'
import type { Translations } from '@/i18n/locales/zh'

import { convertFileNode } from './content-tree-converters'
import type { FileNode } from './content-tree-types'

type ConfirmDialogState = {
  open: boolean
  title: string
  description: string
  onConfirm: () => void
}

type AddToast = ReturnType<typeof useToast>['addToast']

type Input = {
  projectPath: string | null
  node: FileNode
  isDeleting: boolean
  selectedPath: string | null
  setIsDeleting: (value: boolean) => void
  setConfirmDialog: (value: ConfirmDialogState | null) => void
  setTree: (tree: FileNode[]) => void
  setSelectedPath: (path: string | null) => void
  addToast: AddToast
  labels: Translations['tree']
}

async function performDelete(input: Input) {
  if (!input.projectPath || input.isDeleting) return

  input.setIsDeleting(true)
  try {
    if (input.node.kind !== 'dir' && input.node.kind !== 'chapter') {
      return
    }

    const nextTree = await deleteProjectTreeNode(input.projectPath, {
      kind: input.node.kind,
      path: input.node.path,
    })
    input.setTree(nextTree.map(convertFileNode))

    if (input.selectedPath === input.node.path) {
      input.setSelectedPath(null)
    }

    input.setConfirmDialog(null)

    input.addToast({
      title: input.labels.deleteSuccess,
      description: input.labels.deletedDesc.replace('{name}', input.node.title || input.node.name),
      variant: 'success',
    })
  } catch (error) {
    console.error('Failed to delete:', error)
    input.addToast({ title: input.labels.deleteFailed, description: String(error), variant: 'destructive' })
  } finally {
    input.setIsDeleting(false)
  }
}

export function createDeleteConfirm(input: {
  projectPath: string | null
  node: FileNode
  isDeleting: boolean
  selectedPath: string | null
  setIsDeleting: (value: boolean) => void
  setConfirmDialog: (value: ConfirmDialogState | null) => void
  setTree: (tree: FileNode[]) => void
  setSelectedPath: (path: string | null) => void
  addToast: AddToast
  labels: Translations['tree']
}) {
  if (!input.projectPath) {
    input.addToast({ title: input.labels.errorNoProject, description: input.labels.errorNoProject, variant: 'destructive' })
    return null
  }

  if (input.node.kind === 'knowledge' || input.node.kind === 'asset_dir' || input.node.kind === 'asset_file') {
    input.addToast({ title: input.labels.deleteNotSupported, description: input.labels.deleteNotSupportedDesc, variant: 'info' })
    return null
  }

  return {
    open: true,
    title: input.labels.deleteConfirmTitle,
    description: input.labels.deleteConfirmDesc.replace('{name}', input.node.title || input.node.name),
    onConfirm: () => {
      void performDelete({
        projectPath: input.projectPath,
        node: input.node,
        isDeleting: input.isDeleting,
        selectedPath: input.selectedPath,
        setIsDeleting: input.setIsDeleting,
        setConfirmDialog: input.setConfirmDialog,
        setTree: input.setTree,
        setSelectedPath: input.setSelectedPath,
        addToast: input.addToast,
        labels: input.labels,
      })
    },
  }
}
