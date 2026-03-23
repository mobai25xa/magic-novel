import { Download, Edit, FileText, FolderPlus, Plus, Trash2, Upload } from 'lucide-react'

import { CoordinateContextMenu } from '@/components/common/CoordinateContextMenu'
import { ContextMenuItem, ContextMenuSeparator } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

import type { TreeNodeProps } from '../content-tree-types'

type Input = {
  node: TreeNodeProps['node']
  contextMenu: { x: number; y: number } | null
  onClose: () => void
  onCreateChapter?: (volumePath: string) => void
  onOpenRename: () => void
  onOpenCreateFolder: () => void
  onOpenCreateFile: () => void
  onOpenImportManuscript: () => void
  onOpenImportChapter: () => void
  onOpenExport: () => void
  onDelete: () => Promise<void>
}

function isKnowledgeNode(node: TreeNodeProps['node']) {
  return node.path.startsWith('knowledge:') || node.assetRelativePath?.startsWith('.magic_novel/') === true
}

function DirectoryMenu(input: Input) {
  const { translations } = useTranslation()
  const tr = translations.tree
  return (
    <>
      <ContextMenuItem onClick={input.onOpenImportManuscript}>
        <Upload className="mr-2 h-4 w-4" />
        {tr.importToVolume}
      </ContextMenuItem>
      <ContextMenuItem onClick={input.onOpenImportChapter}>
        <Upload className="mr-2 h-4 w-4" />
        {tr.importChapterToVolume}
      </ContextMenuItem>
      <ContextMenuItem onClick={input.onOpenExport}>
        <Download className="mr-2 h-4 w-4" />
        {tr.exportVolume}
      </ContextMenuItem>
    </>
  )
}

function ChapterMenu(input: Input) {
  const { translations } = useTranslation()
  return (
    <ContextMenuItem onClick={input.onOpenExport}>
      <Download className="mr-2 h-4 w-4" />
      {translations.tree.exportChapter}
    </ContextMenuItem>
  )
}

export function TreeNodeContextMenu(input: Input) {
  const { translations } = useTranslation()
  const tr = translations.tree
  if (!input.contextMenu) return null

  return (
    <CoordinateContextMenu x={input.contextMenu.x} y={input.contextMenu.y} onClose={input.onClose} contentClassName="w-56">
      {input.node.kind === 'dir' && input.onCreateChapter ? (
        <>
          <ContextMenuItem
            onClick={() => {
              input.onClose()
              input.onCreateChapter?.(input.node.path)
            }}
          >
            <Plus className="mr-2 h-4 w-4" />
            {tr.newChapter}
          </ContextMenuItem>
          <ContextMenuSeparator />
        </>
      ) : null}

      {input.node.kind === 'knowledge' || input.node.kind === 'asset_dir' ? (
        <>
          <ContextMenuItem onClick={input.onOpenCreateFolder}>
            <FolderPlus className="mr-2 h-4 w-4" />
            {tr.newFolder}
          </ContextMenuItem>
          <ContextMenuItem onClick={input.onOpenCreateFile}>
            <FileText className="mr-2 h-4 w-4" />
            {tr.newFile}
          </ContextMenuItem>
          <ContextMenuSeparator />
        </>
      ) : null}

      {input.node.kind !== 'knowledge' && !isKnowledgeNode(input.node) ? (
        <ContextMenuItem onClick={input.onOpenRename}>
          <Edit className="mr-2 h-4 w-4" />
          {tr.rename}
        </ContextMenuItem>
      ) : null}

      <ContextMenuSeparator />

      {input.node.kind === 'dir' ? <DirectoryMenu {...input} /> : null}
      {input.node.kind === 'chapter' ? <ChapterMenu {...input} /> : null}

      <ContextMenuSeparator />
      <ContextMenuItem
        onClick={() => {
          input.onClose()
          void input.onDelete()
        }}
        destructive
      >
        <Trash2 className="mr-2 h-4 w-4" />
        {tr.delete}
      </ContextMenuItem>
    </CoordinateContextMenu>
  )
}
