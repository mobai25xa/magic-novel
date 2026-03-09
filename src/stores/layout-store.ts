import { useEffect } from 'react'
import { create } from 'zustand'
import { persist } from 'zustand/middleware'

type TocSortField = 'manual' | 'name' | 'createdAt' | 'updatedAt'
type SortOrder = 'asc' | 'desc'

interface LayoutState {
  // Panel visibility
  isLeftPanelVisible: boolean
  isRightPanelVisible: boolean

  // Panel widths
  leftPanelWidth: number
  rightPanelWidth: number

  // TOC sort
  tocSort: {
    field: TocSortField
    order: SortOrder
  }

  // Gamma: Fullscreen mode
  isFullscreen: boolean

  // Actions
  toggleLeftPanel: () => void
  toggleRightPanel: () => void
  setLeftPanelWidth: (width: number) => void
  setRightPanelWidth: (width: number) => void
  setTocSort: (sort: { field: TocSortField; order: SortOrder }) => void
  toggleFullscreen: () => void
}

const MIN_PANEL_WIDTH = 200
const MAX_PANEL_WIDTH = 500
const DEFAULT_LEFT_WIDTH = 280
const DEFAULT_RIGHT_WIDTH = 288

export const useLayoutStore = create<LayoutState>()(
  persist(
    (set) => ({
      isLeftPanelVisible: true,
      isRightPanelVisible: true,
      leftPanelWidth: DEFAULT_LEFT_WIDTH,
      rightPanelWidth: DEFAULT_RIGHT_WIDTH,

      tocSort: {
        field: 'manual',
        order: 'asc',
      },

      // Gamma: Fullscreen
      isFullscreen: false,

      toggleLeftPanel: () =>
        set((state) => ({
          isLeftPanelVisible: !state.isLeftPanelVisible,
        })),

      toggleRightPanel: () =>
        set((state) => ({
          isRightPanelVisible: !state.isRightPanelVisible,
        })),

      setLeftPanelWidth: (width) =>
        set({
          leftPanelWidth: Math.max(MIN_PANEL_WIDTH, Math.min(MAX_PANEL_WIDTH, width)),
        }),

      setRightPanelWidth: (width) =>
        set({
          rightPanelWidth: Math.max(MIN_PANEL_WIDTH, Math.min(MAX_PANEL_WIDTH, width)),
        }),

      setTocSort: (sort) => set({ tocSort: sort }),

      toggleFullscreen: () =>
        set((state) => ({ isFullscreen: !state.isFullscreen })),
    }),
    {
      name: 'magic-novel-layout',
    }
  )
)

export { MIN_PANEL_WIDTH, MAX_PANEL_WIDTH }

/**
 * 编辑器面板响应式自动隐藏 (Dev-B P4)
 * - 窗口宽度 < 1280px：右侧 AI 面板默认隐藏
 * - 窗口宽度 < 1024px：左侧大纲面板也默认隐藏
 */
const layoutStore = useLayoutStore

type EditorPanelAutoHideOptions = {
  disableRightPanelAutoHide?: boolean
}

export function useEditorPanelAutoHide(options: EditorPanelAutoHideOptions = {}) {
  const disableRightPanelAutoHide = options.disableRightPanelAutoHide ?? false

  useEffect(() => {
    const handleResize = () => {
      const width = window.innerWidth
      const state = layoutStore.getState()

      if (width < 1024) {
        if (state.isLeftPanelVisible) layoutStore.setState({ isLeftPanelVisible: false })
        if (!disableRightPanelAutoHide && state.isRightPanelVisible) layoutStore.setState({ isRightPanelVisible: false })
      } else if (width < 1280) {
        if (!disableRightPanelAutoHide && state.isRightPanelVisible) layoutStore.setState({ isRightPanelVisible: false })
      }
    }

    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [disableRightPanelAutoHide])
}
