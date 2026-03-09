import { useEffect, useState } from 'react'

import {
  loadDiscoverWeekStats,
  type DailyStats,
} from '@/features/discover-analytics'
import { scanProjects, type FileNode as BackendFileNode } from '@/features/project-home'
import { eventBus, EVENTS } from '@/lib/events'
import { useSettingsStore } from '@/state/settings'

import type { HomeStats } from './home-page-types'

function countWords(nodes: BackendFileNode[]): number {
  let count = 0
  for (const node of nodes) {
    if (node.kind === 'chapter' && node.text_length_no_whitespace) {
      count += node.text_length_no_whitespace
    } else if (node.kind === 'dir' && node.children) {
      count += countWords(node.children)
    }
  }
  return count
}

function calculateTodayWordDelta(stats: DailyStats[]) {
  return stats.reduce((sum, item) => sum + item.word_count, 0)
}

function normalizeProjectPath(path: string) {
  return path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()
}

async function loadTodayWordDelta(rootDir: string | null) {
  if (!rootDir) {
    return 0
  }

  try {
    const weekStats = await loadDiscoverWeekStats(1, rootDir)
    return calculateTodayWordDelta(weekStats)
  } catch (error) {
    console.error('Failed to load today word delta:', error)
    return 0
  }
}

async function collectStats(projectList: Array<{ path: string }>, rootDir: string | null) {
  let total = 0
  const typeCounts: Record<string, number> = {}
  const genresByProjectPath: Record<string, string[]> = {}

  let snapshots: Array<{
    path: string
    project: { project_type?: string[] }
    tree: BackendFileNode[]
  }> = []

  if (rootDir) {
    try {
      snapshots = await scanProjects(rootDir)
    } catch (error) {
      console.error('Failed to scan projects root for stats:', rootDir, error)
    }
  }

  const snapshotsByPath = new Map(
    snapshots.map((snapshot) => [normalizeProjectPath(snapshot.path), snapshot]),
  )

  for (const project of projectList) {
    const projectPath = String(project.path || '').trim()
    if (!projectPath) {
      continue
    }

    const normalizedProjectPath = normalizeProjectPath(projectPath)
    const snapshot = snapshotsByPath.get(normalizedProjectPath)
    if (!snapshot) {
      continue
    }

    total += countWords(snapshot.tree)

    const genres = snapshot.project.project_type || []
    genresByProjectPath[projectPath] = genres

    for (const genre of genres) {
      typeCounts[genre] = (typeCounts[genre] || 0) + 1
    }
  }

  let popularType: HomeStats['popularType'] = null
  for (const [type, count] of Object.entries(typeCounts)) {
    if (!popularType || count > popularType.count) {
      popularType = { type, count }
    }
  }

  const todayWordDelta = await loadTodayWordDelta(rootDir)

  return { totalWordCount: total, todayWordDelta, popularType, genresByProjectPath }
}

export function useHomePageStats(projectList: Array<{ path: string }>) {
  const projectsRootDir = useSettingsStore((state) => state.projectsRootDir)
  const [refreshTick, setRefreshTick] = useState(0)

  const [stats, setStats] = useState<HomeStats>({
    totalWordCount: 0,
    todayWordDelta: 0,
    popularType: null,
    genresByProjectPath: {},
  })

  useEffect(() => {
    const calculate = async () => {
      if (projectList.length <= 0) {
        setStats({ totalWordCount: 0, todayWordDelta: 0, popularType: null, genresByProjectPath: {} })
        return
      }

      const next = await collectStats(projectList, projectsRootDir)
      setStats(next)
    }

    void calculate()
  }, [projectList, projectsRootDir, refreshTick])

  useEffect(() => {
    const handleRefresh = () => {
      setRefreshTick((value) => value + 1)
    }

    eventBus.on(EVENTS.CHAPTER_SAVED, handleRefresh)
    eventBus.on(EVENTS.STATS_REFRESH_NEEDED, handleRefresh)

    return () => {
      eventBus.off(EVENTS.CHAPTER_SAVED, handleRefresh)
      eventBus.off(EVENTS.STATS_REFRESH_NEEDED, handleRefresh)
    }
  }, [])

  return stats
}
