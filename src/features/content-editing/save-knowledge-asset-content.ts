import type { Editor } from '@tiptap/react'

import { readAssetFile, saveAssetFile } from '@/features/assets-management'
import {
  editorDocToAssetTree,
  type KnowledgeAssetTree,
} from '@/features/assets-management/asset-editor-document'
import { refreshProjectTreeLifecycle } from '@/features/project-lifecycle'

export async function saveKnowledgeAssetContent(input: {
  editor: Editor
  projectPath: string
  assetRelativePath: string
}) {
  const currentAsset = (await readAssetFile(input.projectPath, input.assetRelativePath)) as KnowledgeAssetTree
  const editorDoc = input.editor.getJSON()
  const nextAsset = editorDocToAssetTree(currentAsset, editorDoc)

  await saveAssetFile(input.projectPath, input.assetRelativePath, nextAsset)

  const tree = await refreshProjectTreeLifecycle(input.projectPath)
  return tree
}
