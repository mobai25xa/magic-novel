import {
  createKnowledgeDocument as createKnowledgeDocumentCommand,
  createKnowledgeFolder as createKnowledgeFolderCommand,
  deleteKnowledgeEntry as deleteKnowledgeEntryCommand,
  getKnowledgeTree as getKnowledgeTreeCommand,
  readKnowledgeDocument as readKnowledgeDocumentCommand,
  saveKnowledgeDocument as saveKnowledgeDocumentCommand,
  type KnowledgeDocument,
  type KnowledgeTreeNode,
} from '@/lib/tauri-commands/knowledge-documents'

export type { KnowledgeDocument, KnowledgeTreeNode }

export async function loadKnowledgeTree(projectPath: string): Promise<KnowledgeTreeNode[]> {
  return getKnowledgeTreeCommand(projectPath)
}

export async function readKnowledgeDocument(projectPath: string, virtualPath: string): Promise<KnowledgeDocument> {
  return readKnowledgeDocumentCommand(projectPath, virtualPath)
}

export async function saveKnowledgeDocument(projectPath: string, virtualPath: string, markdown: string): Promise<void> {
  await saveKnowledgeDocumentCommand(projectPath, virtualPath, markdown)
}

export async function createKnowledgeFolder(projectPath: string, parentVirtualDir: string, name: string): Promise<string> {
  return createKnowledgeFolderCommand(projectPath, parentVirtualDir, name)
}

export async function createKnowledgeDocument(projectPath: string, parentVirtualDir: string, name: string): Promise<string> {
  return createKnowledgeDocumentCommand(projectPath, parentVirtualDir, name)
}

export async function deleteKnowledgeEntry(projectPath: string, virtualPath: string): Promise<void> {
  await deleteKnowledgeEntryCommand(projectPath, virtualPath)
}
