import { useSettingsStore, type PlanningGenerationMode } from '@/state/settings'
import type { ProviderType, SettingsState } from '@/stores/settings-types'

export type PlanningGenerationIssueCode =
  | 'planning_model_missing'
  | 'planning_api_key_missing'
  | 'planning_base_url_missing'
  | 'planning_enabled_models_empty'

export type PlanningGenerationSourceTag =
  | 'llm_primary'
  | 'llm_dedicated'
  | 'deterministic_fallback'

export type PlanningGenerationConfig = {
  mode: PlanningGenerationMode
  provider: ProviderType
  model?: string
  baseUrl?: string
  apiKey?: string
  enabledModels: string[]
  sourceTag: PlanningGenerationSourceTag
  hasConfigIssue: boolean
  issueCode: PlanningGenerationIssueCode | null
}

type PlanningSettings = Pick<
  SettingsState,
  | 'providerType'
  | 'openaiBaseUrl'
  | 'openaiApiKey'
  | 'openaiModel'
  | 'openaiEnabledModels'
  | 'planningGenerationMode'
  | 'planningProviderType'
  | 'planningBaseUrl'
  | 'planningApiKey'
  | 'planningModel'
  | 'planningEnabledModels'
>

function asNonEmptyString(input?: string | null) {
  if (typeof input !== 'string') {
    return undefined
  }

  const normalized = input.trim()
  return normalized || undefined
}

function normalizeModels(input: string[]) {
  const seen = new Set<string>()

  return (input || [])
    .map((item) => String(item || '').trim())
    .filter((item) => {
      if (!item || seen.has(item)) {
        return false
      }

      seen.add(item)
      return true
    })
}

function resolvePlanningSettings(settings?: PlanningSettings) {
  return settings ?? useSettingsStore.getState()
}

export function resolvePlanningGenerationMode(settings?: PlanningSettings): PlanningGenerationMode {
  return resolvePlanningSettings(settings).planningGenerationMode === 'dedicated'
    ? 'dedicated'
    : 'follow_primary'
}

export function getPlanningGenerationConfigIssue(settings?: PlanningSettings): PlanningGenerationIssueCode | null {
  const resolvedSettings = resolvePlanningSettings(settings)
  if (resolvePlanningGenerationMode(resolvedSettings) !== 'dedicated') {
    return null
  }

  if (!asNonEmptyString(resolvedSettings.planningBaseUrl)) {
    return 'planning_base_url_missing'
  }

  if (!asNonEmptyString(resolvedSettings.planningApiKey)) {
    return 'planning_api_key_missing'
  }

  const enabledModels = normalizeModels(resolvedSettings.planningEnabledModels)
  if (enabledModels.length === 0) {
    return 'planning_enabled_models_empty'
  }

  if (!asNonEmptyString(resolvedSettings.planningModel)) {
    return 'planning_model_missing'
  }

  return null
}

export function resolvePlanningGenerationConfig(settings?: PlanningSettings): PlanningGenerationConfig {
  const resolvedSettings = resolvePlanningSettings(settings)
  const mode = resolvePlanningGenerationMode(resolvedSettings)
  const issueCode = getPlanningGenerationConfigIssue(resolvedSettings)

  if (mode === 'dedicated') {
    const enabledModels = normalizeModels(resolvedSettings.planningEnabledModels)
    const model = asNonEmptyString(resolvedSettings.planningModel) || enabledModels[0]

    return {
      mode,
      provider: resolvedSettings.planningProviderType,
      model,
      baseUrl: asNonEmptyString(resolvedSettings.planningBaseUrl),
      apiKey: asNonEmptyString(resolvedSettings.planningApiKey),
      enabledModels,
      sourceTag: 'llm_dedicated',
      hasConfigIssue: issueCode !== null,
      issueCode,
    }
  }

  const enabledModels = normalizeModels(resolvedSettings.openaiEnabledModels)
  const model = asNonEmptyString(resolvedSettings.openaiModel) || enabledModels[0]

  return {
    mode,
    provider: resolvedSettings.providerType,
    model,
    baseUrl: asNonEmptyString(resolvedSettings.openaiBaseUrl),
    apiKey: asNonEmptyString(resolvedSettings.openaiApiKey),
    enabledModels,
    sourceTag: 'llm_primary',
    hasConfigIssue: false,
    issueCode: null,
  }
}
