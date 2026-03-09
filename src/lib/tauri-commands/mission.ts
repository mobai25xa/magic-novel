/**
 * mission.ts — Tauri command bindings for the Mission system.
 *
 * Mirrors the Rust commands in src-tauri/src/commands/mission.rs.
 */

import { invoke } from '@tauri-apps/api/core'

// ── Types ────────────────────────────────────────────────────────

export interface Feature {
  id: string
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled'
  description: string
  skill: string
  preconditions: string[]
  depends_on: string[]
  expected_behavior: string[]
  verification_steps: string[]
  write_paths?: string[]
}

export interface WorkerAssignment {
  feature_id: string
  attempt: number
  started_at: number
  last_heartbeat_at: number
}

export interface StateDoc {
  schema_version: number
  mission_id: string
  state: MissionState
  current_feature_id?: string
  current_worker_id?: string
  assignments: Record<string, WorkerAssignment>
  worker_pids: Record<string, number>
  cwd: string
  updated_at: number
}

export interface FeaturesDoc {
  schema_version: number
  mission_id: string
  title: string
  features: Feature[]
}

export interface HandoffEntry {
  feature_id: string
  worker_id: string
  ok: boolean
  summary: string
  commands_run: string[]
  artifacts: string[]
  issues: string[]
}

export type MissionState =
  | 'awaiting_input'
  | 'initializing'
  | 'running'
  | 'paused'
  | 'orchestrator_turn'
  | 'completed'

export interface MissionGetStatusOutput {
  state: StateDoc
  features: FeaturesDoc
  handoffs: HandoffEntry[]
}

export interface MissionCreateInput {
  project_path: string
  title: string
  mission_text: string
  features: Feature[]
}

export interface MissionStartInput {
  project_path: string
  mission_id: string
  max_workers?: number
  model?: string
  provider?: string
  base_url?: string
  api_key?: string
}

export interface MissionCreateOutput {
  schema_version: number
  mission_id: string
}

// ── Commands ─────────────────────────────────────────────────────

export async function missionCreate(
  input: MissionCreateInput,
): Promise<MissionCreateOutput> {
  return invoke<MissionCreateOutput>('mission_create', { input })
}

export async function missionList(projectPath: string): Promise<string[]> {
  return invoke<string[]>('mission_list', { input: { project_path: projectPath } })
}

export async function missionGetStatus(
  projectPath: string,
  missionId: string,
): Promise<MissionGetStatusOutput> {
  return invoke<MissionGetStatusOutput>('mission_get_status', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

export async function missionStart(input: MissionStartInput): Promise<void> {
  return invoke<void>('mission_start', { input })
}

export async function missionPause(
  projectPath: string,
  missionId: string,
): Promise<void> {
  return invoke<void>('mission_pause', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

export async function missionResume(
  projectPath: string,
  missionId: string,
): Promise<void> {
  return invoke<void>('mission_resume', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

export async function missionCancel(
  projectPath: string,
  missionId: string,
): Promise<void> {
  return invoke<void>('mission_cancel', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}
