import type { TreeNodeProps } from '../content-tree-types'

type Input = {
  node: TreeNodeProps['node']
  parentNode?: TreeNodeProps['parentNode']
  isDir: boolean
  dragState: TreeNodeProps['dragState']
  setDragState: TreeNodeProps['setDragState']
  onMoveChapter?: TreeNodeProps['onMoveChapter']
  getSiblingIndex: TreeNodeProps['getSiblingIndex']
}

export function createDragHandlers(input: Input) {
  const handleDragStart = (e: React.DragEvent) => {
    if (input.isDir) {
      e.preventDefault()
      return
    }

    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', input.node.path)
    input.setDragState({ draggingNode: input.node, dropTarget: null })
  }

  const handleDragOver = (e: React.DragEvent, nodeElement?: HTMLDivElement | null) => {
    e.preventDefault()
    e.stopPropagation()

    if (!input.dragState.draggingNode) return
    if (input.dragState.draggingNode.path === input.node.path) return

    e.dataTransfer.dropEffect = 'move'

    const rect = nodeElement?.getBoundingClientRect()
    if (!rect) return

    const y = e.clientY - rect.top
    const height = rect.height

    if (input.isDir) {
      input.setDragState((prev) => ({ ...prev, dropTarget: { node: input.node, position: 'inside' } }))
      return
    }

    const position = y < height / 2 ? 'before' : 'after'
    input.setDragState((prev) => ({ ...prev, dropTarget: { node: input.node, position } }))
  }

  const handleDragLeave = (e: React.DragEvent, nodeElement?: HTMLDivElement | null) => {
    const relatedTarget = e.relatedTarget as HTMLElement
    if (relatedTarget && nodeElement?.contains(relatedTarget)) {
      return
    }

    if (input.dragState.dropTarget?.node.path === input.node.path) {
      input.setDragState((prev) => ({ ...prev, dropTarget: null }))
    }
  }

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()

    if (!input.dragState.draggingNode || !input.dragState.dropTarget || !input.onMoveChapter) {
      setTimeout(() => input.setDragState({ draggingNode: null, dropTarget: null }), 200)
      return
    }

    const { draggingNode, dropTarget } = input.dragState

    if (dropTarget.position === 'inside' && input.isDir) {
      input.onMoveChapter(draggingNode.path, input.node.path, input.node.children?.length || 0)
    } else if (!input.isDir && input.parentNode) {
      const currentIndex = input.getSiblingIndex(input.node, input.parentNode.children)
      const targetIndex = dropTarget.position === 'after' ? currentIndex + 1 : currentIndex
      input.onMoveChapter(draggingNode.path, input.parentNode.path, targetIndex)
    }

    setTimeout(() => input.setDragState({ draggingNode: null, dropTarget: null }), 200)
  }

  return {
    handleDragStart,
    handleDragOver,
    handleDragLeave,
    handleDrop,
  }
}

export function getDropClassName(input: {
  isDropTarget: boolean
  dragState: TreeNodeProps['dragState']
}) {
  if (!input.isDropTarget || !input.dragState.dropTarget) return ''

  if (input.dragState.dropTarget.position === 'inside') return 'bg-success-20'
  if (input.dragState.dropTarget.position === 'before') return 'border-t-2 border-t-primary'
  if (input.dragState.dropTarget.position === 'after') return 'border-b-2 border-b-primary'
  return ''
}
