export type HomeTab = 'home' | 'discover' | 'skills' | 'workers' | 'recycle'

export type HomeEditProject = {
  path: string
  name: string
  author: string
  description?: string
  coverImage?: string
  projectType?: string[]
}

export type HomeConfirmDialog = {
  open: boolean
  title: string
  description: string
}

export type HomePendingAction =
  | { type: 'move_to_recycle'; path: string }
  | { type: 'permanent_delete'; id: string }

export type HomeContextMenu = {
  x: number
  y: number
  projectPath: string
}

export type HomeImportKind = 'manuscript' | 'worldview' | 'outline' | 'character' | 'prompt' | 'lore'

export type HomeStats = {
  totalWordCount: number
  todayWordDelta: number
  popularType: { type: string; count: number } | null
  genresByProjectPath: Record<string, string[]>
}

export type HomeCreateProjectInput = {
  name: string
  author: string
  tags: string
  coverImage?: string
  projectType: string[]
}

export type HomeEditProjectInput = {
  name: string
  author: string
  description?: string
  coverImage?: string
  projectType?: string[]
}

export type HomeControllerState = {
  dialogOpen: boolean
  editDialogOpen: boolean
  editingProject: HomeEditProject | null
  activeTab: HomeTab
  typeFilter: string | null
  contextMenu: HomeContextMenu | null
  ioProjectPath: string | null
  exportDialogOpen: boolean
  importDialogOpen: boolean
  confirmDialog: HomeConfirmDialog | null
  pendingAction: HomePendingAction | null
  isMutating: boolean
}

export type HomeControllerSetters = {
  setDialogOpen: (value: boolean) => void
  setEditDialogOpen: (value: boolean) => void
  setEditingProject: (value: HomeEditProject | null) => void
  setActiveTab: (value: HomeTab) => void
  setTypeFilter: (value: string | null) => void
  setContextMenu: (value: HomeContextMenu | null) => void
  setIoProjectPath: (value: string | null) => void
  setExportDialogOpen: (value: boolean) => void
  setImportDialogOpen: (value: boolean) => void
  setConfirmDialog: (value: HomeConfirmDialog | null) => void
  setPendingAction: (value: HomePendingAction | null) => void
  setIsMutating: (value: boolean) => void
}
