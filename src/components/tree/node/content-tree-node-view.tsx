import { TreeNodeChildren } from './content-tree-node-content'
import type { TreeNodeController } from './content-tree-node-controller'
import { TreeNodeForms } from './content-tree-node-forms'
import { TreeNodeMenu } from './content-tree-node-menu'
import { TreeNodeRow } from './content-tree-node-row'

import type { TreeNodeProps } from '../content-tree-types'

type Input = {
  props: TreeNodeProps
  controller: TreeNodeController
  actions: {
    handleDelete: () => Promise<void>
    handleImportManuscriptHere: () => Promise<void>
    handleImportChapterHere: () => Promise<void>
    handleExport: (format: string) => Promise<void>
    handleRenameConfirm: (newName: string) => Promise<void>
    handleCreateFolder: (title: string) => Promise<void>
    handleCreateFile: (title: string) => Promise<void>
  }
}

function TreeNodeMain(input: Input) {
  const { props, controller } = input

  return (
    <>
      <TreeNodeRow
        node={props.node}
        level={props.level}
        isDir={controller.isDir}
        isExpanded={controller.isExpanded}
        isSelected={controller.isSelected}
        isDragging={controller.isDragging}
        dragClassName={controller.dragClassName}
        onClick={() => {
          if (controller.isDir) {
            controller.toggleExpanded()
          } else {
            props.onSelect(props.node)
          }
        }}
        onContextMenu={(e) => {
          e.preventDefault()
          e.stopPropagation()
          controller.setContextMenu({ x: e.clientX, y: e.clientY })
        }}
        onDragStart={!controller.isDir && !!props.onMoveChapter ? controller.dragHandlers.handleDragStart : undefined}
        onDragOver={
          props.onMoveChapter
            ? (e) => controller.dragHandlers.handleDragOver(e, controller.nodeRef.current)
            : undefined
        }
        onDragLeave={
          props.onMoveChapter
            ? (e) => controller.dragHandlers.handleDragLeave(e, controller.nodeRef.current)
            : undefined
        }
        onDrop={props.onMoveChapter ? controller.dragHandlers.handleDrop : undefined}
        variant={props.variant}
      />

      <TreeNodeChildren
        isDir={controller.isDir}
        isExpanded={controller.isExpanded}
        node={props.node}
        nextLevel={props.level + 1}
        onSelect={props.onSelect}
        selectedPath={props.selectedPath}
        onDelete={props.onDelete}
        onRename={props.onRename}
        onCreateChapter={props.onCreateChapter}
        onMoveChapter={props.onMoveChapter}
        dragState={props.dragState}
        setDragState={props.setDragState}
        getSiblingIndex={props.getSiblingIndex}
        variant={props.variant}
      />
    </>
  )
}

function TreeNodeOverlays(input: Input) {
  return (
    <>
      <TreeNodeForms
        node={input.props.node}
        controller={input.controller}
        actions={input.actions}
      />

      <TreeNodeMenu
        node={input.props.node}
        controller={input.controller}
        onCreateChapter={input.props.onCreateChapter}
        onDelete={input.actions.handleDelete}
      />
    </>
  )
}

export function ContentTreeNodeView(input: Input) {
  return (
    <div>
      <TreeNodeMain {...input} />
      <TreeNodeOverlays {...input} />
    </div>
  )
}
