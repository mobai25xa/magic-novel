import type {
  Actor,
  CommitRequest,
  CommitResult,
  EntityHead,
  ExportRequest,
  ExportResult,
  FetchOpenAiModelsInput,
  OpenAiChatCompletionInput,
  OpenAiModelListResult,
  OpenAiProviderSettings,
  PreviewRequest,
  PreviewResult,
  RecoverOutput,
  RollbackByCallIdInput,
  RollbackByRevisionInput,
  RollbackOutput,
  SaveOpenAiProviderSettingsInput,
} from '@/platform/tauri/clients/agent-versioning-client'
import {
  aiOpenAiChatCompletionClient,
  fetchOpenAiModelsClient,
  getOpenAiProviderSettingsClient,
  jvmCommitPatchClient,
  jvmExportChapterClient,
  jvmPreviewPatchClient,
  saveOpenAiProviderSettingsClient,
  vcGetCurrentHeadClient,
  vcRecoverClient,
  vcRollbackByCallIdClient,
  vcRollbackByRevisionClient,
} from '@/platform/tauri/clients/agent-versioning-client'

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
    openai_embedding_source: input.openai_embedding_source === 'local' ? 'local' : 'provider',
    openai_embedding_detected: detected,
    openai_embedding_detection_reason: reason,
    openai_embedding_enabled: enabled,
  }
}

export async function getOpenAiProviderSettings(): Promise<OpenAiProviderSettings> {
  const settings = await getOpenAiProviderSettingsClient()
  return normalizeOpenAiProviderSettings(settings)
}

export async function saveOpenAiProviderSettings(
  input: SaveOpenAiProviderSettingsInput,
): Promise<OpenAiProviderSettings> {
  const settings = await saveOpenAiProviderSettingsClient(input)
  return normalizeOpenAiProviderSettings(settings)
}

export async function fetchOpenAiModels(input: FetchOpenAiModelsInput): Promise<OpenAiModelListResult> {
  return fetchOpenAiModelsClient(input)
}

export async function aiOpenAiChatCompletion(input: OpenAiChatCompletionInput): Promise<unknown> {
  return aiOpenAiChatCompletionClient(input)
}

export async function jvmExportChapter(input: ExportRequest): Promise<ExportResult> {
  return jvmExportChapterClient(input)
}

export async function jvmPreviewPatch(input: PreviewRequest): Promise<PreviewResult> {
  return jvmPreviewPatchClient(input)
}

export async function jvmCommitPatch(input: CommitRequest): Promise<CommitResult> {
  return jvmCommitPatchClient(input)
}

export async function vcGetCurrentHead(projectPath: string, entityId: string): Promise<EntityHead> {
  return vcGetCurrentHeadClient(projectPath, entityId)
}

export async function vcRollbackByRevision(input: RollbackByRevisionInput): Promise<RollbackOutput> {
  return vcRollbackByRevisionClient(input)
}

export async function vcRollbackByCallId(input: RollbackByCallIdInput): Promise<RollbackOutput> {
  return vcRollbackByCallIdClient(input)
}

export async function vcRecover(projectPath: string): Promise<RecoverOutput> {
  return vcRecoverClient(projectPath)
}

export type {
  Actor,
  CommitRequest,
  CommitResult,
  EntityHead,
  ExportRequest,
  ExportResult,
  FetchOpenAiModelsInput,
  OpenAiChatCompletionInput,
  OpenAiModelListResult,
  OpenAiProviderSettings,
  PreviewRequest,
  PreviewResult,
  RecoverOutput,
  RollbackByCallIdInput,
  RollbackByRevisionInput,
  RollbackOutput,
  SaveOpenAiProviderSettingsInput,
}
