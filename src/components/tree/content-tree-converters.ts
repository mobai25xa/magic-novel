import type { MagicAssetNode } from '@/features/content-tree-management'

import type { BackendFileNode, FileNode } from './content-tree-types'

export function convertMagicAssetNode(node: MagicAssetNode): FileNode {
  if (node.kind === 'dir') {
    return {
      kind: 'asset_dir',
      name: node.name,
      title: node.title || node.name,
      path: `magic_assets/${node.path}`,
      assetRelativePath: node.path,
      children: node.children.map(convertMagicAssetNode),
      updatedAt: undefined,
    }
  }

  const title = node.title || node.name
  return {
    kind: 'asset_file',
    name: node.name,
    title,
    path: `magic_assets/${node.path}`,
    assetRelativePath: node.path,
    updatedAt: node.modified_at,
  }
}

export function convertFileNode(node: BackendFileNode): FileNode {
  return {
    kind: node.kind,
    name: node.name,
    path: node.path,
    children: node.children?.map(convertFileNode),
    chapterId: node.chapter_id,
    title: node.title,
    textLengthNoWhitespace: node.text_length_no_whitespace,
    status: node.status,
    createdAt: node.created_at,
    updatedAt: node.updated_at,
  }
}
