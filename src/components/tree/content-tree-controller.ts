import { useEffect, useMemo, useState } from 'react'

import {
  loadMagicAssetsTree,
  type MagicAssetNode,
} from '@/features/content-tree-management'
import { useToast } from '@/magic-ui/components'
import { useLayoutStore } from '@/stores/layout-store'
import { useProjectStore } from '@/stores/project-store'
import { useTranslation } from '@/hooks/use-translation'

import {
  createHandleMoveChapter,
  createHandleRename,
  createHandleSelect,
} from './content-tree-controller-actions'
import { createDeleteConfirm } from './content-tree-controller-delete'
import { convertMagicAssetNode } from './content-tree-converters'
import { sortTree } from './content-tree-sort'
import type { DragState, FileNode } from './content-tree-types'

type ConfirmDialogState = {
  open: boolean
  title: string
  description: string
  onConfirm: () => void
}

function makeKnowledgeRoot(nodes: MagicAssetNode[], label: string): FileNode {
  return {
    kind: 'knowledge',
    name: label,
    title: label,
    path: 'knowledge',
    children: nodes.map(convertMagicAssetNode),
  }
}

function useKnowledgeTree(projectPath: string | null, label: string, reloadKey: unknown) {
  const [knowledgeTree, setKnowledgeTree] = useState<FileNode | null>(null)

  useEffect(() => {
    const loadKnowledge = async () => {
      if (!projectPath) {
        setKnowledgeTree(null)
        return
      }

      try {
        const nodes = await loadMagicAssetsTree(projectPath)
        setKnowledgeTree(makeKnowledgeRoot(nodes, label))
      } catch (error) {
        console.error('Failed to load magic_assets tree:', error)
        setKnowledgeTree(makeKnowledgeRoot([], label))
      }
    }

    void loadKnowledge()
  }, [projectPath, label, reloadKey])

  return knowledgeTree
}

export function useContentTreeController(input: {
  onChapterSelect: (path: string, chapterId: string, title?: string) => void
  onAssetSelect?: (relativePath: string) => void
  mode?: 'all' | 'knowledge'
  hideKnowledgeRoot?: boolean
}) {
  const { tree, selectedPath, setSelectedPath, projectPath, setTree } = useProjectStore()
  const { tocSort } = useLayoutStore()
  const { addToast } = useToast()
  const { translations } = useTranslation()
  const labels = translations.tree

  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState | null>(null)
  const knowledgeTree = useKnowledgeTree(projectPath, labels.knowledgeBase, tree)
  const [isDeleting, setIsDeleting] = useState(false)
  const [dragState, setDragState] = useState<DragState>({ draggingNode: null, dropTarget: null })

  const dragEnabled = tocSort.field === 'manual'
  const sortedTree = useMemo(() => {
    const base = knowledgeTree ? [knowledgeTree, ...tree] : tree
    const scoped = input.mode === 'knowledge'
      ? base.filter((node) => node.kind === 'knowledge')
      : base

    const normalized = input.mode === 'knowledge' && input.hideKnowledgeRoot
      ? scoped.flatMap((node) => (node.kind === 'knowledge' ? node.children || [] : [node]))
      : scoped

    return sortTree(normalized, tocSort)
  }, [tree, knowledgeTree, tocSort, input.mode, input.hideKnowledgeRoot])

  const getSiblingIndex = (node: FileNode, parentChildren?: FileNode[]) =>
    parentChildren?.findIndex((child) => child.path === node.path) ?? 0

  const handleSelect = createHandleSelect({
    setSelectedPath,
    onChapterSelect: input.onChapterSelect,
    onAssetSelect: input.onAssetSelect,
  })

  const clearConfirmDialog = () => setConfirmDialog(null)

  const handleDelete = (node: FileNode) => {
    const dialog = createDeleteConfirm({
      projectPath,
      node,
      isDeleting,
      selectedPath,
      setIsDeleting,
      setConfirmDialog,
      setTree,
      setSelectedPath,
      addToast,
      labels,
    })

    if (dialog) setConfirmDialog(dialog)
  }

  const store = { setTree, setSelectedPath }
  const handleRename = createHandleRename({ projectPath, addToast, store, labels })
  const handleMoveChapter = createHandleMoveChapter({ projectPath, selectedPath, addToast, store, labels })

  return {
    sortedTree,
    selectedPath,
    confirmDialog,
    isDeleting,
    dragEnabled,
    dragState,
    setDragState,
    handleSelect,
    handleDelete,
    handleRename,
    handleMoveChapter,
    clearConfirmDialog,
    getSiblingIndex,
  }
}
