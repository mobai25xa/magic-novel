import type { HomeCreateProjectInput } from '@/components/home/page/home-page-types'
import type {
  CreateProjectInput,
  ProjectBootstrapStatus,
  StartProjectBootstrapInput,
} from '@/features/project-home'

type BootstrapPhase = ProjectBootstrapStatus['phase']

type CreateProjectResultLike = {
  bootstrapStatus: Pick<ProjectBootstrapStatus, 'phase' | 'recommended_next_action'> | null
  bootstrapError: string | null
  bootstrapUnsupported: boolean
}

export type CreateProjectResultKind =
  | 'scaffold_only'
  | 'bootstrap_unavailable'
  | 'ready_for_review'
  | 'ready_to_write'
  | 'partially_generated'
  | 'failed'

export type CreatePagePhaseTranslationKey =
  | 'phasePending'
  | 'phaseAssemblingPrompt'
  | 'phaseLlmGenerating'
  | 'phaseWritingArtifacts'
  | 'phasePartiallyGenerated'
  | 'phaseReadyForReview'
  | 'phaseReadyToWrite'
  | 'phaseFailed'

export type CreatePageHeadlineTranslationKey =
  | 'resultCreatedOnly'
  | 'resultBootstrapUnavailable'
  | 'resultReadyForReview'
  | 'resultReadyToWrite'
  | 'resultPartial'
  | 'resultFailed'

export type CreatePageRecommendedActionTranslationKey =
  | 'recommendedWaitForBootstrap'
  | 'recommendedReviewBlueprint'
  | 'recommendedReviewProtagonist'
  | 'recommendedAdjustVolumePlan'
  | 'recommendedExpandFirstFiveChapters'
  | 'recommendedStartChapterOne'
  | 'recommendedContinuePlanning'

export const DEFAULT_CREATE_PROJECT_TARGET_REF = 'knowledge:.magic_novel/task/current_bootstrap_task.md'

const BOOTSTRAP_PHASE_TRANSLATION_KEYS: Record<BootstrapPhase, CreatePagePhaseTranslationKey> = {
  pending: 'phasePending',
  assembling_prompt: 'phaseAssemblingPrompt',
  llm_generating: 'phaseLlmGenerating',
  writing_artifacts: 'phaseWritingArtifacts',
  partially_generated: 'phasePartiallyGenerated',
  ready_for_review: 'phaseReadyForReview',
  ready_to_write: 'phaseReadyToWrite',
  failed: 'phaseFailed',
}

function normalizeText(value?: string) {
  const normalized = value?.trim()
  return normalized ? normalized : undefined
}

export function normalizeCreateProjectLabels(input?: string[]) {
  return (input ?? [])
    .map((item) => item.trim())
    .filter(Boolean)
    .filter((item, index, values) => values.indexOf(item) === index)
}

export function deriveTargetWordsPerVolume(targetTotalWords: number, plannedVolumes?: number) {
  if (!plannedVolumes || plannedVolumes <= 0) {
    return undefined
  }

  return Math.max(1, Math.round(targetTotalWords / plannedVolumes))
}

export function buildCreationBrief(data: HomeCreateProjectInput) {
  const projectType = normalizeCreateProjectLabels(data.projectType)
  const tone = normalizeCreateProjectLabels(data.tone)

  return [
    `作品名：${data.name}`,
    `作者：${data.author}`,
    `简介：${data.description}`,
    `题材：${projectType.join(' / ') || '未指定'}`,
    `目标总字数：${data.targetTotalWords}`,
    `预计卷数：${data.plannedVolumes ?? '未指定'}`,
    `每卷目标字数：${deriveTargetWordsPerVolume(data.targetTotalWords, data.plannedVolumes) ?? '未指定'}`,
    `每章目标字数：${data.targetWordsPerChapter ?? '未指定'}`,
    `叙事视角：${normalizeText(data.narrativePov) ?? '未指定'}`,
    `风格倾向：${tone.join(' / ') || '未指定'}`,
    `目标读者：${normalizeText(data.audience) ?? '未指定'}`,
    `主角设定：${normalizeText(data.protagonistSeed) ?? '未提供'}`,
    `对手设定：${normalizeText(data.counterpartSeed) ?? '未提供'}`,
    `世界观种子：${normalizeText(data.worldSeed) ?? '未提供'}`,
    `结局方向：${normalizeText(data.endingDirection) ?? '未提供'}`,
  ].join('\n')
}

export function buildCreateProjectCommandInput(
  projectPath: string,
  data: HomeCreateProjectInput,
): CreateProjectInput {
  const tone = normalizeCreateProjectLabels(data.tone)

  return {
    path: projectPath,
    name: data.name.trim(),
    author: data.author.trim(),
    description: normalizeText(data.description),
    coverImage: normalizeText(data.coverImage),
    projectType: normalizeCreateProjectLabels(data.projectType),
    targetTotalWords: data.targetTotalWords,
    plannedVolumes: data.plannedVolumes,
    targetWordsPerVolume: deriveTargetWordsPerVolume(data.targetTotalWords, data.plannedVolumes),
    targetWordsPerChapter: data.targetWordsPerChapter,
    narrativePov: normalizeText(data.narrativePov),
    tone: tone.length > 0 ? tone : undefined,
    audience: normalizeText(data.audience),
  }
}

export function buildStartProjectBootstrapInput(
  projectPath: string,
  data: HomeCreateProjectInput,
): StartProjectBootstrapInput {
  const tone = normalizeCreateProjectLabels(data.tone)

  return {
    project_path: projectPath,
    creation_brief: buildCreationBrief(data),
    description: normalizeText(data.description),
    target_total_words: data.targetTotalWords,
    planned_volumes: data.plannedVolumes,
    target_words_per_volume: deriveTargetWordsPerVolume(data.targetTotalWords, data.plannedVolumes),
    target_words_per_chapter: data.targetWordsPerChapter,
    narrative_pov: normalizeText(data.narrativePov),
    tone: tone.length > 0 ? tone : undefined,
    audience: normalizeText(data.audience),
    protagonist_seed: normalizeText(data.protagonistSeed),
    counterpart_seed: normalizeText(data.counterpartSeed),
    world_seed: normalizeText(data.worldSeed),
    ending_direction: normalizeText(data.endingDirection),
  }
}

export function resolveCreateProjectResultKind(
  result: CreateProjectResultLike,
): CreateProjectResultKind {
  if (result.bootstrapUnsupported) {
    return 'bootstrap_unavailable'
  }

  if (!result.bootstrapStatus) {
    return result.bootstrapError ? 'failed' : 'scaffold_only'
  }

  switch (result.bootstrapStatus.phase) {
    case 'ready_for_review':
      return 'ready_for_review'
    case 'ready_to_write':
      return 'ready_to_write'
    case 'partially_generated':
      return 'partially_generated'
    case 'failed':
      return 'failed'
    default:
      return result.bootstrapError ? 'failed' : 'partially_generated'
  }
}

export function resolveCreateProjectHeadlineTranslationKey(
  kind: CreateProjectResultKind,
): CreatePageHeadlineTranslationKey {
  switch (kind) {
    case 'bootstrap_unavailable':
      return 'resultBootstrapUnavailable'
    case 'ready_for_review':
      return 'resultReadyForReview'
    case 'ready_to_write':
      return 'resultReadyToWrite'
    case 'partially_generated':
      return 'resultPartial'
    case 'failed':
      return 'resultFailed'
    case 'scaffold_only':
    default:
      return 'resultCreatedOnly'
  }
}

export function resolveBootstrapPhaseTranslationKey(phase?: BootstrapPhase) {
  return phase ? BOOTSTRAP_PHASE_TRANSLATION_KEYS[phase] : undefined
}

export function resolveBootstrapRecommendedActionTranslationKey(
  action?: string,
): CreatePageRecommendedActionTranslationKey {
  switch (action) {
    case 'wait_for_bootstrap':
      return 'recommendedWaitForBootstrap'
    case 'review_blueprint':
      return 'recommendedReviewBlueprint'
    case 'review_protagonist':
    case 'complete_protagonist_profile':
      return 'recommendedReviewProtagonist'
    case 'adjust_volume_plan':
      return 'recommendedAdjustVolumePlan'
    case 'expand_first_five_chapters':
      return 'recommendedExpandFirstFiveChapters'
    case 'start_chapter_one':
      return 'recommendedStartChapterOne'
    default:
      return 'recommendedContinuePlanning'
  }
}

export function resolveBootstrapRecommendedTargetRef(action?: string) {
  switch (action) {
    case 'review_blueprint':
      return 'knowledge:.magic_novel/planning/story_blueprint.md'
    case 'review_protagonist':
    case 'complete_protagonist_profile':
      return 'knowledge:.magic_novel/characters/protagonist.md'
    case 'adjust_volume_plan':
      return 'knowledge:.magic_novel/planning/volume_plan.md'
    case 'expand_first_five_chapters':
      return 'knowledge:.magic_novel/planning/chapter_backlog.md'
    case 'start_chapter_one':
      return 'chapter:bootstrap-v01/bootstrap-v01-c01.json'
    default:
      return null
  }
}

export function resolveCreateProjectTargetRef(result: CreateProjectResultLike) {
  const targetRef = resolveBootstrapRecommendedTargetRef(
    result.bootstrapStatus?.recommended_next_action,
  )
  if (targetRef) {
    return targetRef
  }

  switch (resolveCreateProjectResultKind(result)) {
    case 'ready_for_review':
      return 'knowledge:.magic_novel/planning/story_blueprint.md'
    case 'ready_to_write':
      return 'chapter:bootstrap-v01/bootstrap-v01-c01.json'
    default:
      return DEFAULT_CREATE_PROJECT_TARGET_REF
  }
}

export function shouldAutoEnterCreatedProject(result: CreateProjectResultLike) {
  return !result.bootstrapError
}
