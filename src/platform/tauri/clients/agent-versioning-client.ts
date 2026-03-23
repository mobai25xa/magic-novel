import { invokeTauri } from './core'

export type Actor = 'agent' | 'user' | 'system'

export type EmbeddingSource = 'provider' | 'local'

export interface OpenAiProviderSettings {
  openai_base_url: string
  openai_api_key: string
  openai_model: string
  openai_embedding_model: string
  openai_embedding_base_url: string
  openai_embedding_api_key: string
  openai_local_embedding_base_url: string
  openai_local_embedding_api_key: string
  openai_embedding_source: EmbeddingSource
  openai_embedding_enabled: boolean
  openai_embedding_detected: boolean
  openai_embedding_detection_reason: string
  openai_enabled_models: string[]
}

export interface SaveOpenAiProviderSettingsInput {
  openai_base_url: string
  openai_api_key: string
  openai_model?: string
  openai_embedding_model?: string
  openai_embedding_base_url?: string
  openai_embedding_api_key?: string
  openai_local_embedding_base_url?: string
  openai_local_embedding_api_key?: string
  openai_embedding_source?: EmbeddingSource
  openai_embedding_enabled?: boolean
  openai_embedding_detected?: boolean
  openai_embedding_detection_reason?: string
  openai_enabled_models?: string[]
}

export interface FetchOpenAiModelsInput {
  openai_base_url: string
  openai_api_key: string
}

export interface OpenAiModelListResult {
  models: string[]
}

export interface OpenAiChatCompletionInput {
  messages: unknown[]
  tools?: unknown[]
  tool_choice?: unknown
  model?: string
  temperature?: number
}

export interface EntityHead {
  revision: number
  json_hash: string
  last_call_id?: string
  last_tx_id?: string
  updated_at: number
  last_snapshot_at?: number
}

export interface RollbackByRevisionInput {
  project_path: string
  entity_id: string
  target_revision: number
  call_id: string
  actor: Actor
  reason?: string
}

export interface RollbackByCallIdInput {
  project_path: string
  target_call_id: string
  call_id: string
  actor: Actor
  reason?: string
}

export interface RollbackOutput {
  ok: boolean
  tx_id: string
  revision_before: number
  revision_after: number
  after_hash: string
  rolled_back_to_revision: number
}

export interface RecoverOutput {
  ok: boolean
  repaired_tmp_files: number
  truncated_wal_bytes: number
  rebuilt_head_entities: number
  appended_call_index: number
  notes: string[]
}

function normalizeEmbeddingSource(input: unknown): EmbeddingSource {
  return input === 'local' ? 'local' : 'provider'
}

function normalizeOpenAiProviderSettings(input: OpenAiProviderSettings): OpenAiProviderSettings {
  const model = input.openai_model || 'gpt-4o-mini'
  const enabledModels = Array.isArray(input.openai_enabled_models) ? input.openai_enabled_models : [model]
  const embeddingModel = input.openai_embedding_model || model
  const detected = typeof input.openai_embedding_detected === 'boolean'
    ? input.openai_embedding_detected
    : enabledModels.includes(embeddingModel)
  const reason = (input.openai_embedding_detection_reason || '').trim()
    || (detected ? '' : 'embedding_model_unavailable')
  const enabled = Boolean(input.openai_embedding_enabled) && detected

  return {
    ...input,
    openai_embedding_model: embeddingModel,
    openai_embedding_base_url: input.openai_embedding_base_url || input.openai_base_url,
    openai_embedding_api_key: input.openai_embedding_api_key || input.openai_api_key,
    openai_local_embedding_base_url: input.openai_local_embedding_base_url || 'http://127.0.0.1:11434/v1',
    openai_local_embedding_api_key: input.openai_local_embedding_api_key || '',
    openai_embedding_source: normalizeEmbeddingSource(input.openai_embedding_source),
    openai_embedding_detected: detected,
    openai_embedding_detection_reason: reason,
    openai_embedding_enabled: enabled,
  }
}

export async function getOpenAiProviderSettingsClient(): Promise<OpenAiProviderSettings> {
  const settings = await invokeTauri<OpenAiProviderSettings>('get_openai_provider_settings')
  return normalizeOpenAiProviderSettings(settings)
}

export async function saveOpenAiProviderSettingsClient(
  input: SaveOpenAiProviderSettingsInput,
): Promise<OpenAiProviderSettings> {
  const nextInput = {
    ...input,
    openai_embedding_model: input.openai_embedding_model || input.openai_model,
  }

  const saved = await invokeTauri<OpenAiProviderSettings>('save_openai_provider_settings', { input: nextInput })
  return normalizeOpenAiProviderSettings(saved)
}

export async function fetchOpenAiModelsClient(input: FetchOpenAiModelsInput): Promise<OpenAiModelListResult> {
  return invokeTauri('fetch_openai_models', { input })
}

export async function aiOpenAiChatCompletionClient(input: OpenAiChatCompletionInput): Promise<unknown> {
  return invokeTauri('ai_openai_chat_completion', { input })
}

export async function vcGetCurrentHeadClient(projectPath: string, entityId: string): Promise<EntityHead> {
  return invokeTauri('vc_get_current_head', { projectPath, entityId })
}

export async function vcRollbackByRevisionClient(input: RollbackByRevisionInput): Promise<RollbackOutput> {
  return invokeTauri('vc_rollback_by_revision', { input })
}

export async function vcRollbackByCallIdClient(input: RollbackByCallIdInput): Promise<RollbackOutput> {
  return invokeTauri('vc_rollback_by_call_id', { input })
}

export async function vcRecoverClient(projectPath: string): Promise<RecoverOutput> {
  return invokeTauri('vc_recover', { projectPath })
}
