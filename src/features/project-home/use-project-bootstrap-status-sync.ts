import { useEffect } from 'react'

import { useProjectStore } from '@/state/project'

import { getProjectBootstrapStatusEntry } from './index'
import { isActiveBootstrapStatus, shouldSyncBootstrapStatus } from './bootstrap-status-helpers'

const BOOTSTRAP_STATUS_POLL_INTERVAL_MS = 1200

function isMissingBootstrapStatusError(error: unknown) {
  const message = String(error).toLowerCase()
  return (
    message.includes('not found')
    || message.includes('missing')
    || message.includes('no such file')
  )
}

export function useProjectBootstrapStatusSync(projectPath: string | null) {
  const projectBootstrapState = useProjectStore((state) => state.project?.bootstrapState ?? null)
  const bootstrapStatusProjectPath = useProjectStore((state) => state.bootstrapStatusProjectPath)

  useEffect(() => {
    const store = useProjectStore.getState()

    if (!projectPath) {
      store.clearBootstrapStatus()
      return
    }

    if (!shouldSyncBootstrapStatus({
      projectPath,
      projectBootstrapState,
      bootstrapStatus: store.bootstrapStatus,
      bootstrapStatusProjectPath,
    })) {
      store.clearBootstrapStatus(projectPath)
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

        console.warn('[legacy-bootstrap] Failed to fetch bootstrap status:', error)

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
  }, [bootstrapStatusProjectPath, projectBootstrapState, projectPath])
}
