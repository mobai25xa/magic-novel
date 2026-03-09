import { useMemo, useRef, useState } from 'react'

import type { TreeNodeProps } from '../content-tree-types'

import { createDragHandlers, getDropClassName } from './content-tree-node-dnd'

type DialogState = { open: boolean; kind: 'folder' | 'file' } | null
type ImportDialogState = { open: boolean; kind: 'manuscript' | 'chapter' } | null

export type TreeNodeController = {
  isExpanded: boolean
  isSelected: boolean
  isDir: boolean
  isDragging: boolean
  dragClassName: string
  contextMenu: { x: number; y: number } | null
  renameDialogOpen: boolean
  exportFormatDialogOpen: boolean
  importDialog: ImportDialogState
  createDialog: DialogState
  createFileTitleDialogOpen: boolean
  setContextMenu: (value: { x: number; y: number } | null) => void
  setRenameDialogOpen: (open: boolean) => void
  setExportFormatDialogOpen: (open: boolean) => void
  setImportDialog: (dialog: ImportDialogState) => void
  setCreateDialog: (dialog: DialogState) => void
  setCreateFileTitleDialogOpen: (open: boolean) => void
  toggleExpanded: () => void
  dragHandlers: ReturnType<typeof createDragHandlers>
  nodeRef: React.RefObject<HTMLDivElement | null>
  canShowChildren: boolean
}

export function useContentTreeNodeController(props: TreeNodeProps): TreeNodeController {
  const [isExpanded, setIsExpanded] = useState(true)
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null)
  const [renameDialogOpen, setRenameDialogOpen] = useState(false)
  const [exportFormatDialogOpen, setExportFormatDialogOpen] = useState(false)
  const [importDialog, setImportDialog] = useState<ImportDialogState>(null)
  const [createDialog, setCreateDialog] = useState<DialogState>(null)
  const [createFileTitleDialogOpen, setCreateFileTitleDialogOpen] = useState(false)

  const nodeRef = useRef<HTMLDivElement>(null)
  const isSelected = props.node.path === props.selectedPath
  const isDir = props.node.kind === 'dir' || props.node.kind === 'asset_dir' || props.node.kind === 'knowledge'
  const isDragging = props.dragState.draggingNode?.path === props.node.path
  const isDropTarget = props.dragState.dropTarget?.node.path === props.node.path

  const dragHandlers = useMemo(
    () =>
      createDragHandlers({
        node: props.node,
        parentNode: props.parentNode,
        isDir,
        dragState: props.dragState,
        setDragState: props.setDragState,
        onMoveChapter: props.onMoveChapter,
        getSiblingIndex: props.getSiblingIndex,
      }),
    [
      isDir,
      props.dragState,
      props.getSiblingIndex,
      props.node,
      props.onMoveChapter,
      props.parentNode,
      props.setDragState,
    ],
  )

  return {
    isExpanded,
    isSelected,
    isDir,
    isDragging,
    dragClassName: getDropClassName({ isDropTarget, dragState: props.dragState }),
    contextMenu,
    renameDialogOpen,
    exportFormatDialogOpen,
    importDialog,
    createDialog,
    createFileTitleDialogOpen,
    setContextMenu,
    setRenameDialogOpen,
    setExportFormatDialogOpen,
    setImportDialog,
    setCreateDialog,
    setCreateFileTitleDialogOpen,
    toggleExpanded: () => setIsExpanded((prev) => !prev),
    dragHandlers,
    nodeRef,
    canShowChildren: isDir && isExpanded && !!props.node.children,
  }
}
