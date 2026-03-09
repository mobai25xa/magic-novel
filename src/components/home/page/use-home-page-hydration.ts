import { useEffect } from 'react'

import { listProjectRecycle, scanProjects } from '@/features/project-home'
import { eventBus, EVENTS } from '@/lib/events'

type ReplaceAllProjects = (
  items: Array<{ path: string; name: string; author: string; lastOpenedAt: number; coverImage?: string }>,
  recycled?: Array<{ id: string; path: string; name: string; author: string; deletedAt: number; coverImage?: string }>,
) => void

type ClearAllProjects = () => void

function clearPersistedIfNoRoot(input: {
  projectsRootDir: string | null
  projectListLength: number
  recycledProjectsLength: number
  clearAllProjects: ClearAllProjects
}) {
  if (input.projectsRootDir) return
  if (input.projectListLength <= 0 && input.recycledProjectsLength <= 0) return

  localStorage.removeItem('magic-novel-projects')
  input.clearAllProjects()
}

async function hydrateProjects(projectsRootDir: string, replaceAllProjects: ReplaceAllProjects) {
  const [projects, recycled] = await Promise.all([
    scanProjects(projectsRootDir),
    listProjectRecycle(projectsRootDir),
  ])

  replaceAllProjects(
    projects.map((snapshot) => ({
      path: snapshot.path,
      name: snapshot.project.name,
      author: snapshot.project.author,
      lastOpenedAt: snapshot.project.last_opened_at || snapshot.project.updated_at,
      coverImage: snapshot.project.cover_image,
    })),
    recycled.map((item) => ({
      id: item.id,
      path: item.description,
      name: item.name,
      author: item.origin,
      deletedAt: item.deleted_at,
      coverImage: undefined,
    })),
  )
}

export function useHomePageHydration(input: {
  projectsRootDir: string | null
  projectListLength: number
  recycledProjectsLength: number
  replaceAllProjects: ReplaceAllProjects
  clearAllProjects: ClearAllProjects
}) {
  useEffect(() => {
    clearPersistedIfNoRoot({
      projectsRootDir: input.projectsRootDir,
      projectListLength: input.projectListLength,
      recycledProjectsLength: input.recycledProjectsLength,
      clearAllProjects: input.clearAllProjects,
    })
  }, [
    input.projectsRootDir,
    input.projectListLength,
    input.recycledProjectsLength,
    input.clearAllProjects,
  ])

  useEffect(() => {
    const hydrateFromRootDir = async () => {
      if (!input.projectsRootDir) return

      try {
        await hydrateProjects(input.projectsRootDir, input.replaceAllProjects)
      } catch (error) {
        console.error('Failed to hydrate from root dir:', error)
      }
    }

    void hydrateFromRootDir()

    const handleRefreshRequested = () => {
      void hydrateFromRootDir()
    }

    eventBus.on(EVENTS.RECYCLE_REFRESH_REQUESTED, handleRefreshRequested)

    return () => {
      eventBus.off(EVENTS.RECYCLE_REFRESH_REQUESTED, handleRefreshRequested)
    }
  }, [input.projectsRootDir, input.replaceAllProjects])
}
