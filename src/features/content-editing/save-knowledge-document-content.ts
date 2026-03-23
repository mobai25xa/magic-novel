import type { Editor } from '@tiptap/react'

import { saveKnowledgeDocument } from '@/features/knowledge-documents'
import { operationGetMarkdown } from '@/lib/operations'

export async function saveKnowledgeDocumentContent(input: {
  editor: Editor
  projectPath: string
  knowledgePath: string
}) {
  const markdown = operationGetMarkdown(input.editor)
  await saveKnowledgeDocument(input.projectPath, input.knowledgePath, markdown)
  return null
}
