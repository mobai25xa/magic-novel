import { CreatePage as CreatePageView } from '@/components/create/CreatePage'
import { useProjectStore } from '@/stores/project-store'
import { useNavigationStore } from '@/stores/navigation-store'

export function CreatePage() {
  const setProjectPath = useProjectStore((s) => s.setProjectPath)
  const navigate = useNavigationStore((s) => s.navigate)

  const handleCreated = (path: string) => {
    setProjectPath(path)
    navigate('editor')
  }

  return <CreatePageView onCreated={handleCreated} />
}
