import { HomePageContextMenus } from './page/home-page-context-menus'
import { HomePageDialogs } from './page/home-page-dialogs'

import type { HomePageViewModel } from './home-page-view-model-types'

function openImportDialog(vm: HomePageViewModel, path: string) {
  vm.setters.setContextMenu(null)
  vm.setters.setIoProjectPath(path)
  vm.setters.setImportDialogOpen(true)
}

function openExportDialog(vm: HomePageViewModel, path: string) {
  vm.setters.setContextMenu(null)
  vm.setters.setIoProjectPath(path)
  vm.setters.setExportDialogOpen(true)
}

function restoreProjectByPath(vm: HomePageViewModel, id: string, path: string) {
  void vm.projectActions.handleRestoreProject(id, path)
}

export function HomePageOverlays(input: { vm: HomePageViewModel }) {
  return (
    <>
      <HomePageDialogs
        translations={input.vm.translations}
        state={input.vm.state}
        setters={input.vm.setters}
        onEditProjectConfirm={input.vm.projectActions.handleEditConfirm}
        onConfirmPendingAction={input.vm.handleConfirmPendingAction}
        onImportProject={input.vm.projectActions.handleProjectImport}
        onExportProject={input.vm.projectActions.handleProjectExport}
      />

      <HomePageContextMenus
        contextMenu={input.vm.state.contextMenu}
        recycledProjects={input.vm.projectStore.recycledProjects}
        translations={input.vm.translations}
        setContextMenu={input.vm.setters.setContextMenu}
        onOpenProject={input.vm.projectActions.handleOpenProject}
        onEditProject={input.vm.projectActions.handleEditProject}
        onOpenProjectFolder={input.vm.projectActions.handleOpenProjectFolder}
        onOpenImportDialog={(path) => openImportDialog(input.vm, path)}
        onOpenExportDialog={(path) => openExportDialog(input.vm, path)}
        onDeleteProject={input.vm.projectActions.handleDeleteProjectPending}
        onRestoreProjectByPath={(id, path) => restoreProjectByPath(input.vm, id, path)}
        onPermanentDeleteByPath={input.vm.projectActions.handlePermanentDeletePending}
      />
    </>
  )
}
