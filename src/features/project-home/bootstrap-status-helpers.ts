import type { ProjectBootstrapStatus } from '@/platform/tauri/clients/project-client'

const ACTIVE_BOOTSTRAP_PHASES = new Set<ProjectBootstrapStatus['phase']>([
  'pending',
  'assembling_prompt',
  'llm_generating',
  'writing_artifacts',
])

const TRACKED_BOOTSTRAP_STATES = new Set([
  'bootstrap_running',
  'partially_generated',
  'ready_for_review',
  'ready_to_write',
  'failed',
])

function normalizeBootstrapState(state?: string | null) {
  return state?.trim().toLowerCase() || null
}

export function isActiveBootstrapStatus(status: ProjectBootstrapStatus) {
  return ACTIVE_BOOTSTRAP_PHASES.has(status.phase)
}

export function shouldSyncBootstrapStatus(input: {
  projectPath: string | null
  projectBootstrapState?: string | null
  bootstrapStatus: ProjectBootstrapStatus | null
  bootstrapStatusProjectPath: string | null
}) {
  if (!input.projectPath) {
    return false
  }

  if (
    input.bootstrapStatusProjectPath === input.projectPath
    && input.bootstrapStatus
    && isActiveBootstrapStatus(input.bootstrapStatus)
  ) {
    return true
  }

  const normalizedState = normalizeBootstrapState(input.projectBootstrapState)
  return normalizedState ? TRACKED_BOOTSTRAP_STATES.has(normalizedState) : false
}
