import { lazy, Suspense } from 'react'
import { Spinner } from '@/magic-ui/components'
import { openProjectFlow } from '@/components/home/page/home-page-project-actions-helpers'
import { useNavigationStore } from '@/stores/navigation-store'
import { useProjectStore } from '@/stores/project-store'
import type { AppPage, SettingsSubPage } from '@/types/navigation'

// 首页直接加载，其余页面懒加载
import { WorkspacePage } from '@/pages/workspace'

const DashboardPage = lazy(() =>
  import('@/pages/discover').then((m) => ({ default: m.DiscoverPage }))
)
const SkillsPage = lazy(() =>
  import('@/pages/skills').then((m) => ({ default: m.SkillsPage }))
)
const WorkersPage = lazy(() =>
  import('@/pages/workers').then((m) => ({ default: m.WorkersPage }))
)
const CreatePage = lazy(() =>
  import('@/pages/create').then((m) => ({ default: m.CreatePage }))
)
const ProjectHomePage = lazy(() =>
  import('@/pages/project-home').then((m) => ({ default: m.ProjectHomePage }))
)
const EditorPage = lazy(() =>
  import('@/pages/editor').then((m) => ({ default: m.EditorPage }))
)
const SettingsFullPage = lazy(() =>
  import('@/components/settings/SettingsFullPage').then((m) => ({ default: m.SettingsPage }))
)
const RecyclePage = lazy(() =>
  import('@/pages/recycle').then((m) => ({ default: m.RecyclePage }))
)

interface PageRouterProps {
  currentPage: AppPage
}

function PageFallback() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', flex: 1 }}>
      <Spinner />
    </div>
  )
}

export function PageRouter({ currentPage }: PageRouterProps) {
  const navigate = useNavigationStore((s) => s.navigate)
  const settingsSubPage = useNavigationStore((s) => s.settingsSubPage)
  const setSettingsSubPage = useNavigationStore((s) => s.setSettingsSubPage)
  const goBack = useNavigationStore((s) => s.goBack)
  const projectStore = useProjectStore()

  const handleOpenProject = async (path: string) => {
    try {
      await openProjectFlow({ projectStore, selectedPath: path })
      navigate('project_home')
    } catch (error) {
      console.error('Failed to open project:', error)
    }
  }

  return (
    <Suspense fallback={<PageFallback />}>
      {renderPage(currentPage, {
        onOpenProject: handleOpenProject,
        settingsSubPage,
        setSettingsSubPage,
        goBack,
        navigate,
      })}
    </Suspense>
  )
}

function renderPage(
  page: AppPage,
  ctx: {
    onOpenProject: (path: string) => void
    settingsSubPage: SettingsSubPage
    setSettingsSubPage: (sub: SettingsSubPage) => void
    goBack: () => void
    navigate: (page: AppPage) => void
  },
) {
  switch (page) {
    case 'workspace':
      return <WorkspacePage onOpenSettings={() => ctx.navigate('settings')} />
    case 'dashboard':
      return <DashboardPage />
    case 'skills':
      return <SkillsPage />
    case 'workers':
      return <WorkersPage />
    case 'recycle':
      return <RecyclePage />
    case 'create':
      return <CreatePage />
    case 'project_home':
      return <ProjectHomePage />
    case 'editor':
      return <EditorPage onOpenSettings={() => ctx.navigate('settings')} />
    case 'settings':
      return <SettingsFullPage />
    default:
      return <WorkspacePage onOpenSettings={() => ctx.navigate('settings')} />
  }
}
