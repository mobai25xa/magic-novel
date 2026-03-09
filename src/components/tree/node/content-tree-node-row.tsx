import { ChevronDown, ChevronRight, File, FileText, Folder, FolderOpen } from 'lucide-react'

import { cn } from '@/lib/utils'

import type { TreeNodeProps } from '../content-tree-types'

type Input = {
  node: TreeNodeProps['node']
  level: number
  isDir: boolean
  isExpanded: boolean
  isSelected: boolean
  isDragging: boolean
  dragClassName: string
  variant?: TreeNodeProps['variant']
  onClick: () => void
  onContextMenu: (e: React.MouseEvent) => void
  onDragStart?: (e: React.DragEvent) => void
  onDragOver?: (e: React.DragEvent) => void
  onDragLeave?: (e: React.DragEvent) => void
  onDrop?: (e: React.DragEvent) => void
}

function NodeIcon(input: { isDir: boolean; isExpanded: boolean; variant?: TreeNodeProps['variant'] }) {
  if (!input.isDir) {
    return (
      <>
        <span className="w-4" />
        {input.variant === 'outline' ? (
          <File className="h-4 w-4 shrink-0 editor-shell-outline-item-icon" />
        ) : (
          <FileText className="h-4 w-4 shrink-0 text-info" />
        )}
      </>
    )
  }

  return (
    <>
      {input.isExpanded ? (
        <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
      ) : (
        <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
      )}
      {input.isExpanded ? (
        <FolderOpen className="h-4 w-4 shrink-0 text-warning" />
      ) : (
        <Folder className="h-4 w-4 shrink-0 text-warning" />
      )}
    </>
  )
}

export function TreeNodeRow(input: Input) {
  const outlineVariant = input.variant === 'outline'

  return (
    <div
      onClick={input.onClick}
      onContextMenu={input.onContextMenu}
      draggable={!!input.onDragStart}
      onDragStart={input.onDragStart}
      onDragOver={input.onDragOver}
      onDragLeave={input.onDragLeave}
      onDrop={input.onDrop}
      className={cn(
        outlineVariant
          ? 'editor-shell-outline-item'
          : 'flex items-center gap-1 px-2 py-1 cursor-pointer rounded hover-bg-50',
        !!input.onDragStart && 'cursor-grab active:cursor-grabbing',
        !outlineVariant && input.isSelected && 'active-bg',
        outlineVariant && input.isSelected && 'is-active',
        input.isDragging && 'opacity-50 cursor-grabbing',
        input.dragClassName,
      )}
      style={{ paddingLeft: `${input.level * 12 + 8}px` }}
    >
      {outlineVariant ? (
        <div className="editor-shell-outline-item-left">
          <NodeIcon isDir={input.isDir} isExpanded={input.isExpanded} variant={input.variant} />
          <span className="editor-shell-outline-item-title">{input.node.title || input.node.name}</span>
        </div>
      ) : (
        <>
          <NodeIcon isDir={input.isDir} isExpanded={input.isExpanded} variant={input.variant} />
          <span className="truncate text-sm">{input.node.title || input.node.name}</span>
        </>
      )}

      {!input.isDir && input.node.textLengthNoWhitespace !== undefined ? (
        <span className={cn(outlineVariant ? 'editor-shell-outline-item-count' : 'ml-auto text-xs text-muted-foreground')}>
          {input.node.textLengthNoWhitespace}
        </span>
      ) : null}
    </div>
  )
}
