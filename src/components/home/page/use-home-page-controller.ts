import { useState } from 'react'

import type {
  HomeConfirmDialog,
  HomeContextMenu,
  HomeControllerSetters,
  HomeControllerState,
  HomeEditProject,
  HomePendingAction,
  HomeTab,
} from './home-page-types'

export function useHomePageController() {
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [editingProject, setEditingProject] = useState<HomeEditProject | null>(null)
  const [activeTab, setActiveTab] = useState<HomeTab>('home')
  const [typeFilter, setTypeFilter] = useState<string | null>(null)
  const [contextMenu, setContextMenu] = useState<HomeContextMenu | null>(null)
  const [ioProjectPath, setIoProjectPath] = useState<string | null>(null)
  const [exportDialogOpen, setExportDialogOpen] = useState(false)
  const [importDialogOpen, setImportDialogOpen] = useState(false)
  const [confirmDialog, setConfirmDialog] = useState<HomeConfirmDialog | null>(null)
  const [pendingAction, setPendingAction] = useState<HomePendingAction | null>(null)
  const [isMutating, setIsMutating] = useState(false)

  const state: HomeControllerState = {
    dialogOpen,
    editDialogOpen,
    editingProject,
    activeTab,
    typeFilter,
    contextMenu,
    ioProjectPath,
    exportDialogOpen,
    importDialogOpen,
    confirmDialog,
    pendingAction,
    isMutating,
  }

  const setters: HomeControllerSetters = {
    setDialogOpen,
    setEditDialogOpen,
    setEditingProject,
    setActiveTab,
    setTypeFilter,
    setContextMenu,
    setIoProjectPath,
    setExportDialogOpen,
    setImportDialogOpen,
    setConfirmDialog,
    setPendingAction,
    setIsMutating,
  }

  return { state, setters }
}
