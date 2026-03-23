import type { AssetLibraryNode, KnowledgeTreeNode } from '@/features/content-tree-management'

import type { BackendFileNode, FileNode } from './content-tree-types'

export function convertAssetLibraryNode(node: AssetLibraryNode): FileNode {
  if (node.kind === 'dir') {
    return {
      kind: 'asset_dir',
      name: node.name,
      title: node.title || node.name,
      path: `assets/${node.path}`,
      assetRelativePath: node.path,
      children: node.children.map(convertAssetLibraryNode),
      updatedAt: undefined,
    }
  }

  const title = node.title || node.name
  return {
    kind: 'asset_file',
    name: node.name,
    title,
    path: `assets/${node.path}`,
    assetRelativePath: node.path,
    updatedAt: node.modified_at,
  }
}

export function convertKnowledgeTreeNode(node: KnowledgeTreeNode): FileNode {
  if (node.kind === 'dir') {
    return {
      kind: 'asset_dir',
      name: node.name,
      title: node.title || node.name,
      path: `knowledge:${node.path}`,
      assetRelativePath: node.path,
      children: node.children.map(convertKnowledgeTreeNode),
      updatedAt: undefined,
    }
  }

  const title = node.title || node.name
  return {
    kind: 'asset_file',
    name: node.name,
    title,
    path: `knowledge:${node.path}`,
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
