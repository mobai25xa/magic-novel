import type {
  CreateProjectHandoffDraft,
  InspirationAgentMessage,
  InspirationConsensusState,
  OpenQuestion,
} from '@/features/inspiration/types'

import { invokeTauri } from './core'

export interface InspirationSessionCreateInput {
  title?: string
}

export interface InspirationSessionCreateOutput {
  schema_version: number
  session_id: string
  created_at: number
}

export interface InspirationSessionLoadInput {
  session_id: string
}

export interface InspirationSessionMeta {
  schema_version: number
  session_id: string
  created_at: number
  updated_at: number
  title?: string
  last_turn?: number
  last_stop_reason?: string
  compaction_count?: number
}

export interface InspirationSessionSnapshot {
  meta: InspirationSessionMeta
  messages: InspirationAgentMessage[]
  consensus: InspirationConsensusState
  open_questions: OpenQuestion[]
  final_create_handoff_draft?: CreateProjectHandoffDraft
  runtime_state: string
  hydration_status: string
  last_turn?: number
  next_turn_id?: number
}

export interface InspirationSessionLoadOutput {
  schema_version: number
  session_id: string
  snapshot: InspirationSessionSnapshot
}

export interface InspirationSessionSaveStateInput {
  session_id: string
  consensus: InspirationConsensusState
  open_questions: OpenQuestion[]
  final_create_handoff_draft?: CreateProjectHandoffDraft
}

export interface InspirationSessionSaveStateOutput {
  schema_version: number
  session_id: string
  snapshot: InspirationSessionSnapshot
}

export interface InspirationSessionListInput {
  limit?: number
}

export interface InspirationSessionUpdateMetaInput {
  session_id: string
  title?: string
}

export interface InspirationSessionDeleteInput {
  session_id: string
}

export async function inspirationSessionCreateClient(
  input: InspirationSessionCreateInput,
): Promise<InspirationSessionCreateOutput> {
  return invokeTauri<InspirationSessionCreateOutput>('inspiration_session_create', { input })
}

export async function inspirationSessionLoadClient(
  input: InspirationSessionLoadInput,
): Promise<InspirationSessionLoadOutput> {
  return invokeTauri<InspirationSessionLoadOutput>('inspiration_session_load', { input })
}

export async function inspirationSessionSaveStateClient(
  input: InspirationSessionSaveStateInput,
): Promise<InspirationSessionSaveStateOutput> {
  return invokeTauri<InspirationSessionSaveStateOutput>('inspiration_session_save_state', { input })
}

export async function inspirationSessionListClient(
  input: InspirationSessionListInput = {},
): Promise<InspirationSessionMeta[]> {
  return invokeTauri<InspirationSessionMeta[]>('inspiration_session_list', { input })
}

export async function inspirationSessionUpdateMetaClient(
  input: InspirationSessionUpdateMetaInput,
): Promise<InspirationSessionMeta> {
  return invokeTauri<InspirationSessionMeta>('inspiration_session_update_meta', { input })
}

export async function inspirationSessionDeleteClient(
  input: InspirationSessionDeleteInput,
): Promise<void> {
  await invokeTauri<null>('inspiration_session_delete', { input })
}
