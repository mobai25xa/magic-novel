import { Download, Edit, FolderCog, FolderOpen, RotateCcw, Trash2, Upload } from 'lucide-react'

import { CoordinateContextMenu } from '@/components/common/CoordinateContextMenu'
import { ContextMenuItem, ContextMenuSeparator } from '@/magic-ui/components'

import type { HomeContextMenu } from './home-page-types'

type RecycledProject = {
  id: string
  path: string
}

type Input = {
  contextMenu: HomeContextMenu | null
  recycledProjects: RecycledProject[]
  translations: ReturnType<typeof import('@/hooks/use-translation').useTranslation>['translations']
  setContextMenu: (value: HomeContextMenu | null) => void
  onOpenProject: (path: string) => void
  onEditProject: (path: string) => Promise<void>
  onOpenProjectFolder: (path: string) => void
  onOpenImportDialog: (path: string) => void
  onOpenExportDialog: (path: string) => void
  onDeleteProject: (path: string) => void
  onRestoreProjectByPath: (id: string, path: string) => void
  onPermanentDeleteByPath: (id: string) => void
}

function RecycleContextMenu(input: {
  x: number
  y: number
  translations: Input['translations']
  onClose: () => void
  onRestore: () => void
  onPermanentDelete: () => void
}) {
  return (
    <CoordinateContextMenu x={input.x} y={input.y} onClose={input.onClose} contentClassName="w-56">
      <ContextMenuItem onClick={input.onRestore}>
        <RotateCcw className="mr-2 h-4 w-4" />
        {input.translations.home.restore}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem onClick={input.onPermanentDelete} destructive>
        <Trash2 className="mr-2 h-4 w-4" />
        {input.translations.recyclePage.deletePermanent}
      </ContextMenuItem>
    </CoordinateContextMenu>
  )
}

function ProjectContextMenu(input: {
  x: number
  y: number
  translations: Input['translations']
  onClose: () => void
  onOpenProject: () => void
  onEditProject: () => void
  onOpenProjectFolder: () => void
  onOpenImportDialog: () => void
  onOpenExportDialog: () => void
  onDeleteProject: () => void
}) {
  return (
    <CoordinateContextMenu x={input.x} y={input.y} onClose={input.onClose} contentClassName="w-56">
      <ContextMenuItem onClick={input.onOpenProject}>
        <FolderOpen className="mr-2 h-4 w-4" />
        {input.translations.home.open}
      </ContextMenuItem>
      <ContextMenuItem onClick={input.onEditProject}>
        <Edit className="mr-2 h-4 w-4" />
        {input.translations.common.edit}
      </ContextMenuItem>
      <ContextMenuItem onClick={input.onOpenProjectFolder}>
        <FolderCog className="mr-2 h-4 w-4" />
        {input.translations.home.openFolder}
      </ContextMenuItem>

      <ContextMenuSeparator />

      <ContextMenuItem onClick={input.onOpenImportDialog}>
        <Upload className="mr-2 h-4 w-4" />
        {input.translations.editor.import}
      </ContextMenuItem>
      <ContextMenuItem onClick={input.onOpenExportDialog}>
        <Download className="mr-2 h-4 w-4" />
        {input.translations.editor.export}
      </ContextMenuItem>

      <ContextMenuSeparator />
      <ContextMenuItem onClick={input.onDeleteProject} destructive>
        <Trash2 className="mr-2 h-4 w-4" />
        {input.translations.common.delete}
      </ContextMenuItem>
    </CoordinateContextMenu>
  )
}

export function HomePageContextMenus(input: Input) {
  const projectPath = input.contextMenu?.projectPath
  const isRecycled = projectPath ? input.recycledProjects.some((item) => item.path === projectPath) : false

  return (
    <>
      {input.contextMenu && isRecycled ? (
        <RecycleContextMenu
          x={input.contextMenu.x}
          y={input.contextMenu.y}
          translations={input.translations}
          onClose={() => input.setContextMenu(null)}
          onRestore={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            const item = input.recycledProjects.find((project) => project.path === path)
            if (!item) return
            input.onRestoreProjectByPath(item.id, path)
            input.setContextMenu(null)
          }}
          onPermanentDelete={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            const item = input.recycledProjects.find((project) => project.path === path)
            if (!item) return
            input.onPermanentDeleteByPath(item.id)
            input.setContextMenu(null)
          }}
        />
      ) : null}

      {input.contextMenu && !isRecycled ? (
        <ProjectContextMenu
          x={input.contextMenu.x}
          y={input.contextMenu.y}
          translations={input.translations}
          onClose={() => input.setContextMenu(null)}
          onOpenProject={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            input.setContextMenu(null)
            input.onOpenProject(path)
          }}
          onEditProject={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            void input.onEditProject(path)
          }}
          onOpenProjectFolder={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            input.onOpenProjectFolder(path)
          }}
          onOpenImportDialog={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            input.onOpenImportDialog(path)
          }}
          onOpenExportDialog={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            input.onOpenExportDialog(path)
          }}
          onDeleteProject={() => {
            const path = input.contextMenu?.projectPath
            if (!path) return
            input.setContextMenu(null)
            input.onDeleteProject(path)
          }}
        />
      ) : null}
    </>
  )
}
