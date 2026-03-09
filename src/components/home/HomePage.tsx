import { HomePageLayout } from './home-page-layout'
import { HomePageOverlays } from './home-page-overlays'
import { useHomePageViewModel } from './use-home-page-view-model'

interface HomePageProps {
  onOpenSettings: () => void
}

export function HomePage({ onOpenSettings }: HomePageProps) {
  const vm = useHomePageViewModel(onOpenSettings)

  return (
    <div className="app-page app-page-home">
      <HomePageLayout vm={vm} onOpenSettings={onOpenSettings} />
      <HomePageOverlays vm={vm} />
    </div>
  )
}
