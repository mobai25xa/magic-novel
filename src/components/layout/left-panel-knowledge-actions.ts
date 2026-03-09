import {
  createMagicAssetFileNode,
  createMagicAssetFolderNode,
  getProjectTree,
  type AssetKind,
} from '@/features/content-tree-management'
import { convertFileNode } from './left-panel-types'

type FileNode = {
  kind: 'dir' | 'chapter' | 'knowledge' | 'asset_dir' | 'asset_file'
  name: string
  path: string
  children?: FileNode[]
  chapterId?: string
  title?: string
  textLengthNoWhitespace?: number
  status?: string
  createdAt?: number
  updatedAt?: number
  assetRelativePath?: string
}

type ToastVariant = 'default' | 'success' | 'warning' | 'destructive' | 'info'

type AddToast = (toast: { title: string; description?: string; variant?: ToastVariant }) => void

type KnowledgeDialog =
  | {
      open: true
      kind: 'folder'
      title: string
      placeholder: string
      targetDir: string
      onConfirm: (name: string) => Promise<void>
    }
  | {
      open: true
      kind: 'file-type'
      title: string
      label: string
      targetDir: string
      options: { value: string; label: string }[]
      defaultValue: string
      onConfirm: (assetKind: AssetKind) => void
    }
  | {
      open: true
      kind: 'file-title'
      title: string
      placeholder: string
      targetDir: string
      assetKind: AssetKind
      onConfirm: (name: string) => Promise<void>
    }
  | null

type Translations = {
  tree: {
    newFolderDialogTitle: string
    newFolderPlaceholder: string
    newFileDialogTitle: string
    newFileTypeLabel: string
    newFileDialogPlaceholder: string
    createSuccess: string
    createFolderSuccess: string
    createFileSuccess: string
    createFailed: string
  }
  leftPanel: {
    assetWorldview: string
    assetOutline: string
    assetCharacter: string
    assetLore: string
    assetPrompt: string
  }
}

function findNodeByPath(nodes: FileNode[], targetPath: string): FileNode | null {
  for (const node of nodes) {
    if (node.path === targetPath) return node
    if (!node.children?.length) continue
    const found = findNodeByPath(node.children, targetPath)
    if (found) return found
  }
  return null
}

function resolveTargetDir(tree: FileNode[], selectedPath: string | null): string {
  if (!selectedPath) return ''
  const selected = findNodeByPath(tree, selectedPath)
  if (!selected) return ''

  if (selected.kind === 'asset_dir' && selected.assetRelativePath) {
    return selected.assetRelativePath
  }

  if (selected.kind === 'asset_file' && selected.assetRelativePath) {
    const parts = selected.assetRelativePath.split('/')
    parts.pop()
    return parts.join('/')
  }

  return ''
}

async function refreshProjectTree(projectPath: string, setProjectTree: (nodes: FileNode[]) => void) {
  const tree = await getProjectTree(projectPath)
  setProjectTree(tree.map(convertFileNode))
}

export function createHandleCreateKnowledgeFolder(input: {
  projectPath: string | null
  tree: FileNode[]
  selectedPath: string | null
  setKnowledgeDialog: (dialog: KnowledgeDialog) => void
  setProjectTree: (nodes: FileNode[]) => void
  addToast: AddToast
  translations: Translations
}) {
  return () => {
    if (!input.projectPath) return

    const targetDir = resolveTargetDir(input.tree, input.selectedPath)

    input.setKnowledgeDialog({
      open: true,
      kind: 'folder',
      title: input.translations.tree.newFolderDialogTitle,
      placeholder: input.translations.tree.newFolderPlaceholder,
      targetDir,
      onConfirm: async (name: string) => {
        if (!input.projectPath) return
        try {
          await createMagicAssetFolderNode(input.projectPath, targetDir, name)
          await refreshProjectTree(input.projectPath, input.setProjectTree)
          input.addToast({
            title: input.translations.tree.createSuccess,
            description: input.translations.tree.createFolderSuccess,
            variant: 'success',
          })
          input.setKnowledgeDialog(null)
        } catch (error) {
          console.error('Failed to create knowledge folder:', error)
          input.addToast({
            title: input.translations.tree.createFailed,
            description: String(error),
            variant: 'destructive',
          })
        }
      },
    })
  }
}

export function createHandleCreateKnowledgeFile(input: {
  projectPath: string | null
  tree: FileNode[]
  selectedPath: string | null
  setKnowledgeDialog: (dialog: KnowledgeDialog) => void
  setProjectTree: (nodes: FileNode[]) => void
  addToast: AddToast
  translations: Translations
  onCreated?: (relativePath: string, title: string) => void | Promise<void>
}) {
  return () => {
    if (!input.projectPath) return

    const targetDir = resolveTargetDir(input.tree, input.selectedPath)

    input.setKnowledgeDialog({
      open: true,
      kind: 'file-type',
      title: input.translations.tree.newFileDialogTitle,
      label: input.translations.tree.newFileTypeLabel,
      targetDir,
      options: [
        { value: 'worldview', label: input.translations.leftPanel.assetWorldview },
        { value: 'outline', label: input.translations.leftPanel.assetOutline },
        { value: 'character', label: input.translations.leftPanel.assetCharacter },
        { value: 'lore', label: input.translations.leftPanel.assetLore },
        { value: 'prompt', label: input.translations.leftPanel.assetPrompt },
      ],
      defaultValue: 'worldview',
      onConfirm: (assetKind) => {
        input.setKnowledgeDialog({
          open: true,
          kind: 'file-title',
          title: input.translations.tree.newFileDialogTitle,
          placeholder: input.translations.tree.newFileDialogPlaceholder,
          targetDir,
          assetKind,
          onConfirm: async (name: string) => {
            if (!input.projectPath) return
            try {
              const relativePath = await createMagicAssetFileNode(input.projectPath, targetDir, assetKind, name)
              await refreshProjectTree(input.projectPath, input.setProjectTree)
              await input.onCreated?.(relativePath, name)
              input.addToast({
                title: input.translations.tree.createSuccess,
                description: input.translations.tree.createFileSuccess,
                variant: 'success',
              })
              input.setKnowledgeDialog(null)
            } catch (error) {
              console.error('Failed to create knowledge file:', error)
              input.addToast({
                title: input.translations.tree.createFailed,
                description: String(error),
                variant: 'destructive',
              })
            }
          },
        })
      },
    })
  }
}
