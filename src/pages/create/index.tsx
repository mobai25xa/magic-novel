import { CreatePage as CreatePageView } from '@/components/create/CreatePage'
import { useNavigationStore } from '@/stores/navigation-store'

export function CreatePage() {
  const navigate = useNavigationStore((s) => s.navigate)

  const handleCreated = (_path: string) => {
    navigate('project_home')
  }

  return <CreatePageView onCreated={handleCreated} />
}
