export interface FileNode {
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

export interface DragState {
  draggingNode: FileNode | null
  dropTarget: { node: FileNode; position: 'before' | 'after' | 'inside' } | null
}

export interface PendingAssetWindow extends Window {
  __pendingAssetKind?: string
}

export interface TreeNodeProps {
  node: FileNode
  level: number
  onSelect: (node: FileNode) => void
  selectedPath: string | null
  onDelete: (node: FileNode) => void
  onRename: (node: FileNode, newName: string) => void | Promise<void>
  onCreateChapter?: (volumePath: string) => void
  onMoveChapter?: (chapterPath: string, targetVolumePath: string, targetIndex: number) => void
  dragState: DragState
  setDragState: React.Dispatch<React.SetStateAction<DragState>>
  getSiblingIndex: (node: FileNode, parentChildren?: FileNode[]) => number
  parentNode?: FileNode
  variant?: 'tree' | 'outline'
}

export type BackendFileNode = {
  kind: 'dir' | 'chapter'
  name: string
  path: string
  children?: BackendFileNode[]
  chapter_id?: string
  title?: string
  text_length_no_whitespace?: number
  status?: string
  created_at?: number
  updated_at?: number
}
