import {
  buildStandardAiProviderConfig,
} from '@/features/standard-ai-consumer'
import {
  getPlanningGenerationConfigIssue as getPlanningGenerationConfigIssueFromSettings,
  resolvePlanningGenerationMode,
  type PlanningGenerationIssueCode,
} from '@/features/planning-generation-config'
import {
  createProjectFromIdeation as createProjectFromIdeationCommand,
  type CreateProjectFromIdeationInput,
  type CreateProjectFromIdeationOutput,
  type PlanningManifest,
} from '@/lib/tauri-commands'
import { formatUnknownError } from '@/lib/error-utils'
import { isTauriCommandUnavailableError } from '@/lib/tauri-command-errors'
import type {
  CreateProjectHandoffDraft,
  InspirationConsensusState,
} from '@/features/inspiration/types'
import type { SettingsState } from '@/stores/settings-types'

type PlanningSettingsLike = Pick<
  SettingsState,
  | 'providerType'
  | 'openaiBaseUrl'
  | 'openaiApiKey'
  | 'openaiEnabledModels'
  | 'openaiModel'
  | 'planningGenerationMode'
  | 'planningProviderType'
  | 'planningBaseUrl'
  | 'planningApiKey'
  | 'planningModel'
  | 'planningEnabledModels'
>

type CreateProjectErrorCode =
  | 'MissingMinimumConsensus'
  | 'CoreBundleGenerationFailed'
  | 'PersistenceFailed'
  | 'PlanningProviderConfigurationInvalid'
  | 'create_project_from_ideation_unavailable'

export type PlanningGenerationConfigIssueCode = PlanningGenerationIssueCode

export interface PlanningGenerationConfigIssue {
  code: PlanningGenerationConfigIssueCode
}

export interface CreateProjectErrorSummary {
  code?: CreateProjectErrorCode
  message: string
}

function asRecord(input: unknown): Record<string, unknown> {
  if (input && typeof input === 'object' && !Array.isArray(input)) {
    return input as Record<string, unknown>
  }

  return {}
}

function asString(input: unknown) {
  if (typeof input !== 'string') {
    return undefined
  }

  const normalized = input.trim()
  return normalized || undefined
}

export function getPlanningGenerationConfigIssue(
  settings: PlanningSettingsLike,
): PlanningGenerationConfigIssue | null {
  const issueCode = getPlanningGenerationConfigIssueFromSettings(settings)
  return issueCode ? { code: issueCode } : null
}

export function buildCreateProjectFromIdeationInput(input: {
  projectPath: string
  name: string
  author: string
  consensusSnapshot: InspirationConsensusState
  createHandoff: CreateProjectHandoffDraft
  sessionId?: string | null
}): CreateProjectFromIdeationInput {
  const projectName = input.name.trim()
  const summary = input.createHandoff.description.trim()
  const createHandoff = {
    ...input.createHandoff,
    name: projectName,
    description: summary,
  }

  return {
    path: input.projectPath,
    name: projectName,
    author: input.author.trim(),
    consensusSnapshot: input.consensusSnapshot,
    createHandoff,
    originInspirationSessionId: input.sessionId?.trim() || undefined,
  }
}

export async function createProjectFromIdeation(input: {
  projectPath: string
  name: string
  author: string
  consensusSnapshot: InspirationConsensusState
  createHandoff: CreateProjectHandoffDraft
  sessionId?: string | null
}): Promise<CreateProjectFromIdeationOutput> {
  return createProjectFromIdeationCommand(
    buildCreateProjectFromIdeationInput(input),
  )
}

export function resolvePlanningConfigIssueForCreate(
  settings: PlanningSettingsLike,
): PlanningGenerationConfigIssue | null {
  const issue = getPlanningGenerationConfigIssue(settings)
  if (issue) {
    return issue
  }

  if (resolvePlanningGenerationMode(settings) === 'follow_primary') {
    buildStandardAiProviderConfig({ settings })
  }

  return null
}

export function summarizeCreateProjectError(error: unknown): CreateProjectErrorSummary {
  const errorRecord = asRecord(error)
  const details = asRecord(errorRecord.details)
  const message = asString(errorRecord.message)
    || asString(errorRecord.error)
    || formatUnknownError(error)
  const code = asString(errorRecord.code) || asString(details.code)

  if (code === 'MissingMinimumConsensus'
    || code === 'CoreBundleGenerationFailed'
    || code === 'PersistenceFailed'
    || code === 'PlanningProviderConfigurationInvalid') {
    return {
      code,
      message,
    }
  }

  if (isTauriCommandUnavailableError(error, 'create_project_from_ideation')) {
    return {
      code: 'create_project_from_ideation_unavailable',
      message,
    }
  }

  return { message }
}

export function resolveRecommendedPlanningTarget(
  planningManifest: PlanningManifest | null | undefined,
) {
  const recommended = planningManifest?.recommended_next_doc?.trim()
  if (!recommended) {
    return null
  }

  if (recommended.startsWith('knowledge:') || recommended.startsWith('chapter:') || recommended.startsWith('asset:')) {
    return recommended
  }

  if (recommended.startsWith('.magic_novel/')) {
    return `knowledge:${recommended}`
  }

  return `knowledge:.magic_novel/${recommended.replace(/^\/+/, '')}`
}

export type {
  CreateProjectFromIdeationOutput,
  PlanningManifest,
}
