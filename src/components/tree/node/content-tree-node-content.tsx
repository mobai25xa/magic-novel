import type { TreeNodeProps } from '../content-tree-types'

import { ContentTreeNode } from './content-tree-node'

type Input = {
  isDir: boolean
  isExpanded: boolean
  node: TreeNodeProps['node']
  nextLevel: number
  onSelect: TreeNodeProps['onSelect']
  selectedPath: TreeNodeProps['selectedPath']
  onDelete: TreeNodeProps['onDelete']
  onRename: TreeNodeProps['onRename']
  onCreateChapter: TreeNodeProps['onCreateChapter']
  onMoveChapter: TreeNodeProps['onMoveChapter']
  dragState: TreeNodeProps['dragState']
  setDragState: TreeNodeProps['setDragState']
  getSiblingIndex: TreeNodeProps['getSiblingIndex']
  variant?: TreeNodeProps['variant']
}

export function TreeNodeChildren(input: Input) {
  if (!input.isDir || !input.isExpanded || !input.node.children) {
    return null
  }

  return (
    <div>
      {input.node.children.map((child) => (
        <ContentTreeNode
          key={child.path}
          node={child}
          level={input.nextLevel}
          onSelect={input.onSelect}
          selectedPath={input.selectedPath}
          onDelete={input.onDelete}
          onRename={input.onRename}
          onCreateChapter={input.onCreateChapter}
          onMoveChapter={input.onMoveChapter}
          dragState={input.dragState}
          setDragState={input.setDragState}
          getSiblingIndex={input.getSiblingIndex}
          parentNode={input.node}
          variant={input.variant}
        />
      ))}
    </div>
  )
}
