import { EditorPage as EditorPageView } from '@/components/editor/EditorPage'

interface EditorPageProps {
  onOpenSettings: () => void
}

export function EditorPage({ onOpenSettings }: EditorPageProps) {
  return <EditorPageView onOpenSettings={onOpenSettings} />
}
