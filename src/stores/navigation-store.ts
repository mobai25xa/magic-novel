import { useEffect, useRef } from 'react'
import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { AppPage, NavigationState, SettingsSubPage } from '@/types/navigation'
import { useProjectStore } from '@/stores/project-store'

interface NavigationStore extends NavigationState {
  navigate: (page: AppPage) => void
  setSettingsSubPage: (sub: SettingsSubPage) => void
  toggleSidebar: () => void
  setSidebarCollapsed: (collapsed: boolean) => void
  goBack: () => void
}

export const useNavigationStore = create<NavigationStore>()(
  persist(
    (set, get) => ({
      currentPage: 'workspace' as AppPage,
      previousPage: null as AppPage | null,
      settingsSubPage: 'about' as SettingsSubPage,
      sidebarCollapsed: false,

      navigate: (page: AppPage) => {
        const state = get()

        // editor 需要 projectPath 非空，否则忽略
        if (page === 'editor' || page === 'project_home') {
          const { projectPath } = useProjectStore.getState()
          if (!projectPath) return
        }

        // 进入 settings 时记录 previousPage 以便返回
        if (page === 'settings') {
          set({
            currentPage: page,
            previousPage: state.currentPage !== 'settings' ? state.currentPage : state.previousPage,
          })
          return
        }

        set({ currentPage: page })
      },

      setSettingsSubPage: (sub: SettingsSubPage) => {
        set({ settingsSubPage: sub })
      },

      toggleSidebar: () => {
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed }))
      },

      setSidebarCollapsed: (collapsed: boolean) => {
        set({ sidebarCollapsed: collapsed })
      },

      goBack: () => {
        const { previousPage } = get()
        set({
          currentPage: previousPage ?? 'workspace',
          previousPage: null,
        })
      },
    }),
    {
      name: 'magic-novel-navigation',
      partialize: (state) => ({
        sidebarCollapsed: state.sidebarCollapsed,
      }),
    }
  )
)

const AUTO_COLLAPSE_BREAKPOINT = 1280

/**
 * 窗口宽度 < 1280px 时自动折叠 sidebar，
 * 宽度 >= 1280px 时恢复用户手动设置的状态。
 */
export function useSidebarAutoCollapse() {
  const setSidebarCollapsed = useNavigationStore((s) => s.setSidebarCollapsed)
  const userPreferenceRef = useRef(useNavigationStore.getState().sidebarCollapsed)

  useEffect(() => {
    const handleResize = () => {
      const isNarrow = window.innerWidth < AUTO_COLLAPSE_BREAKPOINT
      if (isNarrow) {
        setSidebarCollapsed(true)
      } else {
        setSidebarCollapsed(userPreferenceRef.current)
      }
    }

    // 记录用户手动切换的偏好
    const unsubscribe = useNavigationStore.subscribe((state, prev) => {
      if (state.sidebarCollapsed !== prev.sidebarCollapsed && window.innerWidth >= AUTO_COLLAPSE_BREAKPOINT) {
        userPreferenceRef.current = state.sidebarCollapsed
      }
    })

    window.addEventListener('resize', handleResize)
    handleResize() // 初始检查

    return () => {
      window.removeEventListener('resize', handleResize)
      unsubscribe()
    }
  }, [setSidebarCollapsed])
}
