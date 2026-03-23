import type { FileNode } from '@/lib/tauri-commands'

export type UiTreeNode = {
  kind: 'dir' | 'chapter' | 'knowledge' | 'asset_dir' | 'asset_file'
  name: string
  path: string
  children?: UiTreeNode[]
  chapterId?: string
  title?: string
  textLengthNoWhitespace?: number
  status?: string
  updatedAt?: number
  assetRelativePath?: string
}

function normalizeAssetRelativePath(path: string, kind: UiTreeNode['kind']) {
  if (kind !== 'asset_dir' && kind !== 'asset_file') return undefined
  const normalized = String(path || '').replace(/\\/g, '/')
  return normalized.startsWith('assets/') ? normalized.slice('assets/'.length) : normalized
}

export function mapBackendTreeNode(node: FileNode): UiTreeNode {
  return {
    kind: node.kind,
    name: node.name,
    path: node.path,
    children: node.children?.map(mapBackendTreeNode),
    chapterId: node.chapter_id,
    title: node.title,
    textLengthNoWhitespace: node.word_count ?? node.text_length_no_whitespace,
    status: node.status,
    updatedAt: node.updated_at,
    assetRelativePath: normalizeAssetRelativePath(node.path, node.kind),
  }
}
