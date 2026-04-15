import { lazy, Suspense, useEffect, useRef } from 'react'

import { openProjectFlow } from '@/components/home/page/home-page-project-actions-helpers'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useSettingsStore } from '@/state/settings'
import { useNavigationStore, useSidebarAutoCollapse } from '@/stores/navigation-store'

import { Spinner, TooltipProvider } from './magic-ui/components'
import { useCloseProtection } from './hooks/use-close-protection'
import { useTheme } from './hooks/use-theme'
import { applyThemeColors } from './lib/theme-utils'

import { GlobalSidebar } from './components/layout/GlobalSidebar'
import { GlobalTopBar } from './components/layout/GlobalTopBar'
import { PageRouter } from './components/layout/PageRouter'

const EditorPage = lazy(() =>
  import('./pages/editor').then((m) => ({ default: m.EditorPage }))
)

/** 编辑器使用独立布局，不走 GlobalSidebar */
const EDITOR_INDEPENDENT = true

function AppFallback() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', minHeight: '100vh' }}>
      <Spinner />
    </div>
  )
}

function App() {
  const { projectPath } = useProjectStore()
  const { reset: resetEditor } = useEditorStore()
  const { customThemeColors } = useSettingsStore()
  const prevProjectPathRef = useRef<string | null>(null)

  const currentPage = useNavigationStore((s) => s.currentPage)
  const sidebarCollapsed = useNavigationStore((s) => s.sidebarCollapsed)
  const navigate = useNavigationStore((s) => s.navigate)
  const toggleSidebar = useNavigationStore((s) => s.toggleSidebar)

  const handleOpenProject = async (path: string) => {
    try {
      await openProjectFlow({
        projectStore: useProjectStore.getState(),
        selectedPath: path,
      })
      navigate('project_home')
    } catch (error) {
      console.error('Failed to open project from sidebar:', error)
    }
  }

  // 应用主题
  useTheme()

  // 响应式 sidebar 自动折叠
  useSidebarAutoCollapse()

  // === Beta: Close Protection START ===
  useCloseProtection()

  // beforeunload 兜底
  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      const { isDirty } = useEditorStore.getState()
      if (isDirty) {
        e.preventDefault()
        e.returnValue = ''
      }
    }
    window.addEventListener('beforeunload', handleBeforeUnload)
    return () => window.removeEventListener('beforeunload', handleBeforeUnload)
  }, [])
  // === Beta: Close Protection END ===

  // 应用自定义颜色
  useEffect(() => {
    applyThemeColors(customThemeColors)
  }, [customThemeColors])

  // 当项目切换时，清空编辑器状态
  useEffect(() => {
    if (prevProjectPathRef.current !== projectPath) {
      resetEditor()
      prevProjectPathRef.current = projectPath
    }
  }, [projectPath, resetEditor])

  // 编辑器页面使用独立布局（保持现有 TopBar + LeftPanel + RightPanel）
  const isEditorPage = EDITOR_INDEPENDENT && currentPage === 'editor'
  const isSettingsPage = currentPage === 'settings'

  if (isEditorPage) {
    return (
      <TooltipProvider>
        <Suspense fallback={<AppFallback />}>
          <EditorPage onOpenSettings={() => navigate('settings')} />
        </Suspense>
      </TooltipProvider>
    )
  }

  // 设置页独享整页布局，不嵌入 GlobalSidebar / GlobalTopBar
  if (isSettingsPage) {
    return (
      <TooltipProvider>
        <div className="app-page app-page-home">
          <PageRouter currentPage={currentPage} />
        </div>
      </TooltipProvider>
    )
  }

  // 其余页面：GlobalSidebar + GlobalTopBar + PageRouter
  return (
    <TooltipProvider>
      <div className="app-page app-page-home" style={{ flexDirection: 'row' }}>
        <GlobalSidebar
          currentPage={currentPage}
          collapsed={sidebarCollapsed}
          onNavigate={navigate}
          onOpenProject={handleOpenProject}
          onToggleCollapse={toggleSidebar}
        />
        <div className="main-wrapper">
          <GlobalTopBar
            currentPage={currentPage}
            onToggleSidebar={toggleSidebar}
            onSearch={() => {}}
            onOpenCreate={() => navigate('create')}
            onOpenSkillCreate={() => {
              // TODO: replace with dedicated create-skill flow
              navigate('settings')
            }}
          />
          <main className="main-scroll">
            <PageRouter currentPage={currentPage} />
          </main>
        </div>
      </div>
    </TooltipProvider>
  )
}

export default App
