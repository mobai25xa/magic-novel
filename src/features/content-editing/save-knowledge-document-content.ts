import type { Editor } from '@tiptap/react'

import { saveKnowledgeDocument } from '@/features/knowledge-documents'
import { refreshPlanningManifestEntry } from '@/features/project-home'
import { operationGetMarkdown } from '@/lib/operations'
import { useProjectStore } from '@/stores/project-store'

export async function saveKnowledgeDocumentContent(input: {
  editor: Editor
  projectPath: string
  knowledgePath: string
}) {
  const markdown = operationGetMarkdown(input.editor)
  await saveKnowledgeDocument(input.projectPath, input.knowledgePath, markdown)
  const planningManifest = await refreshPlanningManifestEntry(input.projectPath)
  useProjectStore.getState().setPlanningManifest(input.projectPath, planningManifest)
  return null
}
