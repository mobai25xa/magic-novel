import { WorkspacePage as WorkspacePageView } from '@/components/home/WorkspacePage'
import { openProjectFlow } from '@/components/home/page/home-page-project-actions-helpers'
import { useNavigationStore } from '@/stores/navigation-store'
import { useProjectStore } from '@/stores/project-store'

interface WorkspacePageProps {
  onOpenSettings: () => void
}

export function WorkspacePage({ onOpenSettings }: WorkspacePageProps) {
  const navigate = useNavigationStore((s) => s.navigate)
  const projectStore = useProjectStore()

  const handleOpenProject = async (path: string) => {
    try {
      await openProjectFlow({ projectStore, selectedPath: path })
      navigate('editor')
    } catch (error) {
      console.error('Failed to open project:', error)
    }
  }

  return (
    <WorkspacePageView
      onOpenProject={handleOpenProject}
      onOpenCreate={() => navigate('create')}
      onOpenSettings={onOpenSettings}
    />
  )
}
