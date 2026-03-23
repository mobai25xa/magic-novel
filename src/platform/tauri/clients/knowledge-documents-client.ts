import { invokeTauri } from './core'

export type KnowledgeTreeNode =
  | { kind: 'dir'; name: string; path: string; title?: string; children: KnowledgeTreeNode[] }
  | { kind: 'file'; name: string; path: string; title?: string; modified_at?: number }

export interface KnowledgeDocument {
  path: string
  title: string
  markdown: string
  content: unknown
}

export async function getKnowledgeTreeClient(projectPath: string): Promise<KnowledgeTreeNode[]> {
  return invokeTauri('get_knowledge_tree', { projectPath })
}

export async function readKnowledgeDocumentClient(
  projectPath: string,
  virtualPath: string,
): Promise<KnowledgeDocument> {
  return invokeTauri('read_knowledge_document', { projectPath, virtualPath })
}

export async function saveKnowledgeDocumentClient(
  projectPath: string,
  virtualPath: string,
  markdown: string,
): Promise<void> {
  return invokeTauri('save_knowledge_document', { projectPath, virtualPath, markdown })
}

export async function createKnowledgeFolderClient(
  projectPath: string,
  parentVirtualDir: string,
  name: string,
): Promise<string> {
  return invokeTauri('create_knowledge_folder', { projectPath, parentVirtualDir, name })
}

export async function createKnowledgeDocumentClient(
  projectPath: string,
  parentVirtualDir: string,
  name: string,
): Promise<string> {
  return invokeTauri('create_knowledge_document', { projectPath, parentVirtualDir, name })
}

export async function deleteKnowledgeEntryClient(
  projectPath: string,
  virtualPath: string,
): Promise<void> {
  return invokeTauri('delete_knowledge_entry', { projectPath, virtualPath })
}
