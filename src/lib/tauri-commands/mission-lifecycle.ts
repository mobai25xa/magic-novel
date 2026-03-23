/**
 * mission-lifecycle.ts — Tauri command bindings for Mission lifecycle helpers.
 *
 * Keep this separate from mission.ts to avoid pushing that file beyond the
 * 1000-line governance threshold during fix_M5 work.
 */

import { invoke } from '@tauri-apps/api/core'

import type { MissionStartInput } from './mission'

export interface MissionLifecycleControlInput {
  project_path: string
  mission_id: string
}

function toMissionLifecycleControlInput(
  projectPath: string,
  missionId: string,
): MissionLifecycleControlInput {
  return {
    project_path: projectPath,
    mission_id: missionId,
  }
}

export async function missionInterrupt(projectPath: string, missionId: string): Promise<void> {
  return invoke<void>('mission_interrupt', {
    input: toMissionLifecycleControlInput(projectPath, missionId),
  })
}

export type MissionResumeWithConfigInput = MissionStartInput

export async function missionResumeWithConfig(input: MissionResumeWithConfigInput): Promise<void> {
  return invoke<void>('mission_resume_with_config', { input })
}

export async function missionRecover(projectPath: string, missionId: string): Promise<void> {
  return invoke<void>('mission_recover', {
    input: toMissionLifecycleControlInput(projectPath, missionId),
  })
}
