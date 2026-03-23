import { useEffect } from 'react'

import { useProjectStore } from '@/state/project'

import { getProjectBootstrapStatusEntry, type ProjectBootstrapStatus } from './index'

const ACTIVE_BOOTSTRAP_PHASES = new Set<ProjectBootstrapStatus['phase']>([
  'pending',
  'assembling_prompt',
  'llm_generating',
  'writing_artifacts',
])

const BOOTSTRAP_STATUS_POLL_INTERVAL_MS = 1200

function isActiveBootstrapStatus(status: ProjectBootstrapStatus) {
  return ACTIVE_BOOTSTRAP_PHASES.has(status.phase)
}

function isMissingBootstrapStatusError(error: unknown) {
  const message = String(error).toLowerCase()
  return (
    message.includes('not found')
    || message.includes('missing')
    || message.includes('no such file')
  )
}

export function useProjectBootstrapStatusSync(projectPath: string | null) {
  useEffect(() => {
    if (!projectPath) {
      useProjectStore.getState().clearBootstrapStatus()
      return
    }

    let disposed = false
    let timerId: number | null = null

    const scheduleNextPoll = () => {
      if (disposed) {
        return
      }

      timerId = window.setTimeout(() => {
        void refreshStatus()
      }, BOOTSTRAP_STATUS_POLL_INTERVAL_MS)
    }

    const refreshStatus = async () => {
      try {
        const status = await getProjectBootstrapStatusEntry(projectPath)
        if (disposed) {
          return
        }

        useProjectStore.getState().setBootstrapStatus(projectPath, status)
        if (isActiveBootstrapStatus(status)) {
          scheduleNextPoll()
        }
      } catch (error) {
        if (disposed) {
          return
        }

        if (isMissingBootstrapStatusError(error)) {
          useProjectStore.getState().clearBootstrapStatus(projectPath)
          return
        }

        console.warn('[project-bootstrap] Failed to fetch bootstrap status:', error)

        const store = useProjectStore.getState()
        if (
          store.bootstrapStatusProjectPath === projectPath
          && store.bootstrapStatus
          && isActiveBootstrapStatus(store.bootstrapStatus)
        ) {
          scheduleNextPoll()
        }
      }
    }

    void refreshStatus()

    return () => {
      disposed = true
      if (timerId !== null) {
        window.clearTimeout(timerId)
      }
    }
  }, [projectPath])
}
