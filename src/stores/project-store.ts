import { create } from 'zustand'
import { persist } from 'zustand/middleware'

interface Project {
  projectId: string
  name: string
  author: string
  description?: string
  createdAt: number
  updatedAt: number
}

interface ProjectListItem {
  path: string
  name: string
  author: string
  lastOpenedAt: number
  coverImage?: string
}

interface RecycledProject {
  id: string
  path: string
  name: string
  author: string
  deletedAt: number
  coverImage?: string
}

interface FileNode {
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

export interface ProjectSnapshot {
  project: {
    project_id: string
    name: string
    author: string
  }
  path: string
}

interface ProjectState {
  // Current project
  projectPath: string | null
  project: Project | null
  tree: FileNode[]
  selectedPath: string | null
  
  // Project list (recent projects)
  projectList: ProjectListItem[]
  
  // Recycle bin
  recycledProjects: RecycledProject[]
  
  // Actions
  setProjectPath: (path: string | null) => void
  setProject: (project: Project | null) => void
  setTree: (tree: FileNode[]) => void
  setSelectedPath: (path: string | null) => void
  addToProjectList: (item: ProjectListItem) => void
  removeFromProjectList: (path: string) => void
  replaceRecycledProjects: (items: RecycledProject[]) => void
  removeProjectFromList: (path: string) => void
  addProjectToList: (item: ProjectListItem) => void
  removeRecycledProjectById: (id: string) => void
  clearAllProjects: () => void
  replaceAllProjects: (items: ProjectListItem[], recycled?: RecycledProject[]) => void
  reset: () => void
}

export const useProjectStore = create<ProjectState>()(
  persist(
    (set) => ({
      projectPath: null,
      project: null,
      tree: [],
      selectedPath: null,
      projectList: [],
      recycledProjects: [],
      
      setProjectPath: (projectPath) => set({ projectPath }),
      setProject: (project) => set({ project }),
      setTree: (tree) => set({ tree }),
      setSelectedPath: (selectedPath) => set({ selectedPath }),
      
      addToProjectList: (item) => set((state) => {
        const filtered = state.projectList.filter(p => p.path !== item.path)
        return { projectList: [item, ...filtered].slice(0, 20) } // Keep max 20 recent projects
      }),
      
      removeFromProjectList: (path) => set((state) => ({
        projectList: state.projectList.filter(p => p.path !== path)
      })),

      replaceRecycledProjects: (items) => set({ recycledProjects: items }),

      removeProjectFromList: (path) => set((state) => ({
        projectList: state.projectList.filter((p) => p.path !== path),
      })),

      addProjectToList: (item) => set((state) => {
        const filtered = state.projectList.filter((p) => p.path !== item.path)
        return { projectList: [item, ...filtered].slice(0, 20) }
      }),

      removeRecycledProjectById: (id) => set((state) => ({
        recycledProjects: state.recycledProjects.filter((item) => item.id !== id),
      })),
      
      clearAllProjects: () => set({ 
        projectPath: null, 
        project: null, 
        tree: [], 
        selectedPath: null,
        projectList: [],
        recycledProjects: []
      }),

      replaceAllProjects: (items, recycled = []) => set({
        projectPath: null,
        project: null,
        tree: [],
        selectedPath: null,
        projectList: items,
        recycledProjects: recycled,
      }),
      
      reset: () => set({ projectPath: null, project: null, tree: [], selectedPath: null }),
    }),
    {
      name: 'magic-novel-projects',
      partialize: (state) => ({ 
        projectList: state.projectList,
        recycledProjects: state.recycledProjects 
      }),
    }
  )
)
