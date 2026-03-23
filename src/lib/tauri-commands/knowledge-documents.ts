import type {
  KnowledgeDocument,
  KnowledgeTreeNode,
} from '@/platform/tauri/clients/knowledge-documents-client'
import {
  createKnowledgeDocumentClient,
  createKnowledgeFolderClient,
  deleteKnowledgeEntryClient,
  getKnowledgeTreeClient,
  readKnowledgeDocumentClient,
  saveKnowledgeDocumentClient,
} from '@/platform/tauri/clients/knowledge-documents-client'

export async function getKnowledgeTree(projectPath: string): Promise<KnowledgeTreeNode[]> {
  return getKnowledgeTreeClient(projectPath)
}

export async function readKnowledgeDocument(
  projectPath: string,
  virtualPath: string,
): Promise<KnowledgeDocument> {
  return readKnowledgeDocumentClient(projectPath, virtualPath)
}

export async function saveKnowledgeDocument(
  projectPath: string,
  virtualPath: string,
  markdown: string,
): Promise<void> {
  return saveKnowledgeDocumentClient(projectPath, virtualPath, markdown)
}

export async function createKnowledgeFolder(
  projectPath: string,
  parentVirtualDir: string,
  name: string,
): Promise<string> {
  return createKnowledgeFolderClient(projectPath, parentVirtualDir, name)
}

export async function createKnowledgeDocument(
  projectPath: string,
  parentVirtualDir: string,
  name: string,
): Promise<string> {
  return createKnowledgeDocumentClient(projectPath, parentVirtualDir, name)
}

export async function deleteKnowledgeEntry(
  projectPath: string,
  virtualPath: string,
): Promise<void> {
  return deleteKnowledgeEntryClient(projectPath, virtualPath)
}

export type { KnowledgeDocument, KnowledgeTreeNode }
