import { InputDialog } from '@/components/common/InputDialog'
import { RenameDialog } from '@/components/common/RenameDialog'
import { SelectDialog } from '@/components/common/SelectDialog'
import { useTranslation } from '@/hooks/use-translation'

import type { TreeNodeProps } from '../content-tree-types'

type DialogState = { open: boolean; kind: 'folder' | 'file' } | null
type ImportDialogState = { open: boolean; kind: 'manuscript' | 'chapter' } | null

type Input = {
  node: TreeNodeProps['node']
  renameDialogOpen: boolean
  createDialog: DialogState
  createFileTitleDialogOpen: boolean
  exportFormatDialogOpen: boolean
  importDialog: ImportDialogState
  setRenameDialogOpen: (open: boolean) => void
  setCreateDialog: (dialog: DialogState) => void
  setCreateFileTitleDialogOpen: (open: boolean) => void
  setExportFormatDialogOpen: (open: boolean) => void
  setImportDialog: (dialog: ImportDialogState) => void
  onRenameConfirm: (name: string) => Promise<void>
  onCreateFolder: (title: string) => Promise<void>
  onCreateFile: (title: string) => Promise<void>
  onExport: (format: string) => Promise<void>
  onImportManuscriptHere: () => Promise<void>
  onImportChapterHere: () => Promise<void>
}

function ImportExportDialogs(input: Input) {
  const { translations } = useTranslation()
  const tr = translations.tree
  if (input.node.kind !== 'dir' && input.node.kind !== 'chapter') {
    return null
  }

  return (
    <>
      <SelectDialog
        open={input.exportFormatDialogOpen}
        title={tr.exportDialogTitle}
        label={tr.exportFormatLabel}
        options={[
          { value: 'md', label: 'Markdown (.md)' },
          { value: 'txt', label: 'Text (.txt)' },
        ]}
        defaultValue="md"
        onClose={() => input.setExportFormatDialogOpen(false)}
        onConfirm={(format) => {
          void input.onExport(format)
        }}
      />

      <SelectDialog
        open={!!input.importDialog?.open}
        title={tr.importDialogTitle}
        label={tr.importTypeLabel}
        options={[
          { value: 'manuscript', label: tr.importTypeManuscript },
          { value: 'chapter', label: tr.importTypeSingleChapter },
        ]}
        defaultValue={input.importDialog?.kind || 'manuscript'}
        onClose={() => input.setImportDialog(null)}
        onConfirm={(value) => {
          input.setImportDialog(null)
          if (value === 'manuscript') {
            void input.onImportManuscriptHere()
          } else {
            void input.onImportChapterHere()
          }
        }}
      />
    </>
  )
}

export function TreeNodeDialogs(input: Input) {
  const { translations } = useTranslation()
  const tr = translations.tree
  return (
    <>
      <RenameDialog
        open={input.renameDialogOpen}
        title={`${tr.renameTo} "${input.node.title || input.node.name}"`}
        defaultValue={input.node.title || input.node.name}
        onClose={() => input.setRenameDialogOpen(false)}
        onConfirm={input.onRenameConfirm}
      />

      <InputDialog
        open={!!input.createDialog?.open && input.createDialog.kind === 'folder'}
        title={tr.newFolderDialogTitle}
        placeholder={tr.newFolderPlaceholder}
        onClose={() => input.setCreateDialog(null)}
        onConfirm={input.onCreateFolder}
      />

      <InputDialog
        open={input.createFileTitleDialogOpen}
        title={tr.newFileDialogTitle}
        placeholder={tr.newFileDialogPlaceholder}
        onClose={() => input.setCreateFileTitleDialogOpen(false)}
        onConfirm={async (title) => {
          await input.onCreateFile(title)
          input.setCreateFileTitleDialogOpen(false)
        }}
      />

      <ImportExportDialogs {...input} />
    </>
  )
}
