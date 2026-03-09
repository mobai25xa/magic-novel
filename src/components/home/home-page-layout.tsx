import { HomePageHeader } from './page/home-page-header'
import { HomePageMainContent } from './page/home-page-main-content'
import { HomePageSidebar } from './page/home-page-sidebar'

import type { HomePageViewModel } from './home-page-view-model-types'

export function HomePageLayout(input: {
  vm: HomePageViewModel
  onOpenSettings: () => void
}) {
  return (
    <>
      <HomePageHeader
        createProjectLabel={input.vm.translations.home.createProject}
        settingsTitle={input.vm.translations.settings.title}
        onOpenCreateProject={() => input.vm.setters.setDialogOpen(true)}
        onOpenSettings={input.onOpenSettings}
      />

      <div className="flex-1 flex overflow-hidden">
        <HomePageSidebar
          activeTab={input.vm.state.activeTab}
          homeLabel={input.vm.translations.home.homePage}
          discoverLabel={input.vm.translations.home.discover}
          skillsLabel={input.vm.translations.home.skills}
          workersLabel={input.vm.translations.home.workers}
          recycleLabel={input.vm.translations.home.recycleBin}
          setActiveTab={input.vm.setters.setActiveTab}
        />

        <HomePageMainContent
          activeTab={input.vm.state.activeTab}
          projectList={input.vm.projectStore.projectList}
          filteredProjects={input.vm.derived.filteredProjects}
          recycledProjects={input.vm.projectStore.recycledProjects}
          totalWordCount={input.vm.stats.totalWordCount}
          todayWordDelta={input.vm.stats.todayWordDelta}
          popularType={input.vm.stats.popularType}
          typeFilter={input.vm.state.typeFilter}
          translations={input.vm.translations}
          onToggleTypeFilter={(type) => {
            input.vm.setters.setTypeFilter(input.vm.state.typeFilter === type ? null : type)
          }}
          onOpenProject={input.vm.projectActions.handleOpenProject}
          onProjectContextMenu={input.vm.projectActions.handleProjectContextMenu}
          onRestoreProject={input.vm.projectActions.handleRestoreProject}
        />
      </div>
    </>
  )
}
