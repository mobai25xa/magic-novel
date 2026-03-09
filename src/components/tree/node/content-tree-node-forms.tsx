import { TreeNodeDialogs } from './content-tree-node-dialogs'
import type { TreeNodeController } from './content-tree-node-controller'

import type { TreeNodeProps } from '../content-tree-types'

type Input = {
  node: TreeNodeProps['node']
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

export function TreeNodeForms(input: Input) {
  return (
    <TreeNodeDialogs
      node={input.node}
      renameDialogOpen={input.controller.renameDialogOpen}
      createDialog={input.controller.createDialog}
      createFileTitleDialogOpen={input.controller.createFileTitleDialogOpen}
      exportFormatDialogOpen={input.controller.exportFormatDialogOpen}
      importDialog={input.controller.importDialog}
      setRenameDialogOpen={input.controller.setRenameDialogOpen}
      setCreateDialog={input.controller.setCreateDialog}
      setCreateFileTitleDialogOpen={input.controller.setCreateFileTitleDialogOpen}
      setExportFormatDialogOpen={input.controller.setExportFormatDialogOpen}
      setImportDialog={input.controller.setImportDialog}
      onRenameConfirm={input.actions.handleRenameConfirm}
      onCreateFolder={input.actions.handleCreateFolder}
      onCreateFile={input.actions.handleCreateFile}
      onExport={input.actions.handleExport}
      onImportManuscriptHere={input.actions.handleImportManuscriptHere}
      onImportChapterHere={input.actions.handleImportChapterHere}
    />
  )
}
