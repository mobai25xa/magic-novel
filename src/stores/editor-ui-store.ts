import { create } from 'zustand'

export type LeftPanelTab = 'outline' | 'knowledge'

function normalizeSidebarPath(path: string) {
  return path.trim().replace(/\\/g, '/')
}

export interface EditorUiState {
  leftPanelTab: LeftPanelTab
  setLeftPanelTab: (tab: LeftPanelTab) => void

  sidebarTreeKnownDirPaths: Record<string, true>
  sidebarTreeCollapsedDirPaths: Record<string, true>
  registerSidebarTreeDirPath: (path: string) => void
  toggleSidebarTreeDirCollapsed: (path: string) => void
  setSidebarTreeCollapsedDirPaths: (paths: string[]) => void
  resetSessionUiState: () => void
}

export const useEditorUiStore = create<EditorUiState>((set) => ({
  leftPanelTab: 'outline',
  setLeftPanelTab: (tab) => set({ leftPanelTab: tab }),

  sidebarTreeKnownDirPaths: {},
  sidebarTreeCollapsedDirPaths: {},
  registerSidebarTreeDirPath: (rawPath) => {
    const path = normalizeSidebarPath(rawPath)
    if (!path) return
    set((state) => {
      if (state.sidebarTreeKnownDirPaths[path]) {
        return {}
      }
      return { sidebarTreeKnownDirPaths: { ...state.sidebarTreeKnownDirPaths, [path]: true } }
    })
  },
  toggleSidebarTreeDirCollapsed: (rawPath) => {
    const path = normalizeSidebarPath(rawPath)
    if (!path) return
    set((state) => {
      const next = { ...state.sidebarTreeCollapsedDirPaths }
      if (next[path]) {
        delete next[path]
      } else {
        next[path] = true
      }
      return { sidebarTreeCollapsedDirPaths: next }
    })
  },
  setSidebarTreeCollapsedDirPaths: (paths) => {
    const next: Record<string, true> = {}
    for (const rawPath of paths) {
      const path = normalizeSidebarPath(rawPath)
      if (path) next[path] = true
    }
    set({ sidebarTreeCollapsedDirPaths: next })
  },
  resetSessionUiState: () =>
    set({
      leftPanelTab: 'outline',
      sidebarTreeKnownDirPaths: {},
      sidebarTreeCollapsedDirPaths: {},
    }),
}))
