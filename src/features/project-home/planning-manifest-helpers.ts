import type { PlanningDocEntry, PlanningManifest } from '@/platform/tauri/clients/project-client'

const PLANNING_DOC_DISPLAY_NAMES: Record<string, string> = {
  story_brief: '故事简报',
  story_blueprint: '故事蓝图',
  narrative_contract: '叙事合同',
  character_cards: '角色卡',
  foreshadow_registry: '伏笔登记表',
  chapter_planning: '章节规划',
  volume_plan: '卷规划',
}

export function normalizePlanningTargetRef(raw: string | null | undefined) {
  const value = raw?.trim()
  if (!value) {
    return null
  }

  if (value.startsWith('knowledge:') || value.startsWith('chapter:') || value.startsWith('asset:')) {
    return value
  }

  if (value.startsWith('.magic_novel/')) {
    return `knowledge:${value}`
  }

  return `knowledge:.magic_novel/${value.replace(/^\/+/, '')}`
}

export function resolvePrimaryContractTarget(manifest: PlanningManifest | null | undefined) {
  const primaryContract = manifest?.docs.find((doc) => doc.id === 'narrative_contract')?.path
    ?? manifest?.docs.find((doc) => doc.required_for_create)?.path
    ?? manifest?.docs[0]?.path
    ?? manifest?.recommended_next_doc
    ?? null

  return normalizePlanningTargetRef(primaryContract)
}

export function resolveRecommendedPlanningTarget(manifest: PlanningManifest | null | undefined) {
  return normalizePlanningTargetRef(manifest?.recommended_next_doc)
}

export function resolvePlanningDocDisplayName(raw: string | null | undefined) {
  const value = raw?.trim()
  if (!value) {
    return ''
  }

  const byPath = value.match(/\/([^/]+)\.md$/)?.[1]
  if (byPath && PLANNING_DOC_DISPLAY_NAMES[byPath]) {
    return PLANNING_DOC_DISPLAY_NAMES[byPath]
  }

  return PLANNING_DOC_DISPLAY_NAMES[value] ?? value
}

export function resolvePlanningEntryDisplayName(doc: PlanningDocEntry) {
  return resolvePlanningDocDisplayName(doc.id) || resolvePlanningDocDisplayName(doc.path) || doc.id
}

export function isPlanningDocReady(doc: PlanningDocEntry) {
  return doc.materialization_state === 'ready'
}

export function isPlanningDocConfirmed(doc: PlanningDocEntry) {
  return doc.approval_state === 'user_refined' || doc.approval_state === 'accepted'
}

export function resolveBundleTone(bundleStatus: string) {
  if (bundleStatus === 'ready') {
    return 'success' as const
  }

  if (bundleStatus === 'failed') {
    return 'warning' as const
  }

  if (bundleStatus === 'ready_for_write') {
    return 'success' as const
  }

  if (bundleStatus === 'missing_core_docs') {
    return 'warning' as const
  }

  return 'info' as const
}
