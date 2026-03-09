import { TreeNodeContextMenu } from './content-tree-node-context-menu'
import type { TreeNodeController } from './content-tree-node-controller'

import type { TreeNodeProps } from '../content-tree-types'

type Input = {
  node: TreeNodeProps['node']
  controller: TreeNodeController
  onCreateChapter: TreeNodeProps['onCreateChapter']
  onDelete: () => Promise<void>
}

export function TreeNodeMenu(input: Input) {
  return (
    <TreeNodeContextMenu
      node={input.node}
      contextMenu={input.controller.contextMenu}
      onClose={() => input.controller.setContextMenu(null)}
      onCreateChapter={input.onCreateChapter}
      onOpenRename={() => {
        input.controller.setContextMenu(null)
        input.controller.setRenameDialogOpen(true)
      }}
      onOpenCreateFolder={() => {
        input.controller.setContextMenu(null)
        input.controller.setCreateDialog({ open: true, kind: 'folder' })
      }}
      onOpenCreateFile={() => {
        input.controller.setContextMenu(null)
        input.controller.setCreateDialog({ open: true, kind: 'file' })
      }}
      onOpenImportManuscript={() => {
        input.controller.setContextMenu(null)
        input.controller.setImportDialog({ open: true, kind: 'manuscript' })
      }}
      onOpenImportChapter={() => {
        input.controller.setContextMenu(null)
        input.controller.setImportDialog({ open: true, kind: 'chapter' })
      }}
      onOpenExport={() => {
        input.controller.setContextMenu(null)
        input.controller.setExportFormatDialogOpen(true)
      }}
      onDelete={input.onDelete}
    />
  )
}
