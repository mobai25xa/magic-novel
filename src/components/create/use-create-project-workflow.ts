import { useCallback, useMemo, useRef, useState } from 'react'

import {
  createProjectFromIdeation,
  resolvePlanningConfigIssueForCreate,
  summarizeCreateProjectError,
  type CreateProjectFromIdeationOutput,
} from '@/features/project-creation'
import { useTranslation } from '@/hooks/use-translation'
import { useToast } from '@/magic-ui/components'
import { useProjectStore } from '@/stores/project-store'
import { useSettingsStore } from '@/stores/settings-store'

import {
  createDefaultProjectDraft,
  validateCreateProjectDraft,
} from './form-utils'
import {
  applyCreateHandoffToDraft,
  buildCreateHandoffFromConsensus,
} from './inspiration/inspiration-helpers'
import { useInspirationWorkflow } from './inspiration/use-inspiration-workflow'
import type {
  CreateProjectDraft,
  CreateProjectFormErrors,
  CreateProjectWorkflowStage,
} from './types'

interface UseCreateProjectWorkflowInput {
  onOpenSettings: () => void
  onClose: () => void
  onProjectReady: (path: string) => void | Promise<void>
  active?: boolean
}

export function useCreateProjectWorkflow(input: UseCreateProjectWorkflowInput) {
  const { translations } = useTranslation()
  const cp = translations.createPage
  const { addToast } = useToast()
  const projectStore = useProjectStore()
  const projectsRootDir = useSettingsStore((state) => state.projectsRootDir)
  const projectGenres = useSettingsStore((state) => state.projectGenres)
  const inspiration = useInspirationWorkflow({ enabled: input.active ?? true })

  const runIdRef = useRef(0)

  const [draft, setDraft] = useState<CreateProjectDraft>(() => createDefaultProjectDraft())
  const [errors, setErrors] = useState<CreateProjectFormErrors>({})
  const [stage, setStage] = useState<CreateProjectWorkflowStage>('ideation')
  const [submitting, setSubmitting] = useState(false)
  const [preserveInspirationSession, setPreserveInspirationSession] = useState(false)

  const resetWorkflow = useCallback((options?: { suppressInspirationAutoCreate?: boolean }) => {
    runIdRef.current += 1
    setDraft(createDefaultProjectDraft())
    setErrors({})
    setStage('ideation')
    setSubmitting(false)
    setPreserveInspirationSession(false)
    inspiration.reset({
      suspendAutoCreate: Boolean(options?.suppressInspirationAutoCreate),
    })
  }, [inspiration])

  const updateDraft = useCallback((patch: Partial<CreateProjectDraft>) => {
    setDraft((current) => ({ ...current, ...patch }))
    setErrors((current) => {
      if (Object.keys(current).length === 0) {
        return current
      }

      const next = { ...current }
      if (patch.name !== undefined) delete next.name
      if (patch.author !== undefined) delete next.author
      if (patch.description !== undefined) delete next.description
      return next
    })

    const handoffPatch: Record<string, string> = {}
    if (patch.name !== undefined) {
      handoffPatch.name = patch.name
    }
    if (patch.description !== undefined) {
      handoffPatch.description = patch.description
    }
    if (Object.keys(handoffPatch).length > 0) {
      inspiration.updateFinalDraft(handoffPatch)
    }
  }, [inspiration])

  const toggleGenre = useCallback((genre: string) => {
    setDraft((current) => ({
      ...current,
      selectedGenres: current.selectedGenres.includes(genre)
        ? current.selectedGenres.filter((item) => item !== genre)
        : [...current.selectedGenres, genre],
    }))
  }, [])

  const enterCreatedProject = useCallback(async (outcome: CreateProjectFromIdeationOutput) => {
    if (!preserveInspirationSession && inspiration.sessionId) {
      await inspiration.deleteSession(inspiration.sessionId)
    }

    projectStore.setProjectPath(outcome.project_snapshot.path)
    projectStore.setProject({
      projectId: outcome.project_snapshot.project.project_id,
      name: outcome.project_snapshot.project.name,
      author: outcome.project_snapshot.project.author,
      description: outcome.project_snapshot.project.description,
      createdAt: outcome.project_snapshot.project.created_at,
      updatedAt: outcome.project_snapshot.project.updated_at,
    })
    projectStore.setTree([])
    projectStore.clearBootstrapStatus()
    projectStore.setPlanningManifest(
      outcome.project_snapshot.path,
      outcome.planning_manifest,
    )
    projectStore.addToProjectList({
      path: outcome.project_snapshot.path,
      name: outcome.project_snapshot.project.name,
      author: outcome.project_snapshot.project.author,
      lastOpenedAt: Date.now(),
      coverImage: outcome.project_snapshot.project.cover_image,
    })

    resetWorkflow()
    await input.onProjectReady(outcome.project_snapshot.path)
  }, [inspiration, input, preserveInspirationSession, projectStore, resetWorkflow])

  const prepareLaunchSheet = useCallback((candidateHandoff?: typeof inspiration.finalCreateHandoffDraft) => {
    const resolvedHandoff = buildCreateHandoffFromConsensus(
      inspiration.consensus,
      candidateHandoff ?? inspiration.finalCreateHandoffDraft,
    )

    setDraft((current) => ({
      ...applyCreateHandoffToDraft(current, resolvedHandoff, projectGenres),
      author: current.author,
    }))
    inspiration.updateFinalDraft(resolvedHandoff)
    setStage('launch_sheet')
  }, [inspiration, projectGenres])

  const handleSubmit = useCallback(async () => {
    const validationErrors = validateCreateProjectDraft(draft)
    if (Object.keys(validationErrors).length > 0) {
      setErrors(validationErrors)
      return
    }

    if (!projectsRootDir) {
      addToast({
        title: translations.common.error,
        description: translations.home.configureRootDir,
        variant: 'destructive',
      })
      input.onOpenSettings()
      return
    }

    const planningConfigIssue = resolvePlanningConfigIssueForCreate(
      useSettingsStore.getState() as ReturnType<typeof useSettingsStore.getState> & Record<string, unknown>,
    )

    if (planningConfigIssue) {
      const issueMessageMap = {
        planning_model_missing: cp.planningModelMissing,
        planning_api_key_missing: cp.planningApiKeyMissing,
        planning_base_url_missing: cp.planningBaseUrlMissing,
        planning_enabled_models_empty: cp.planningEnabledModelsEmpty,
      } as const

      addToast({
        title: translations.common.error,
        description: issueMessageMap[planningConfigIssue.code],
        variant: 'destructive',
      })
      input.onOpenSettings()
      return
    }

    setSubmitting(true)
    setStage('generating_contract')
    const runId = runIdRef.current + 1
    runIdRef.current = runId

    try {
      const createHandoff = buildCreateHandoffFromConsensus(
        inspiration.consensus,
        {
          ...(inspiration.finalCreateHandoffDraft ?? buildCreateHandoffFromConsensus(inspiration.consensus)),
          name: draft.name,
          description: draft.description,
        },
      )

      const outcome = await createProjectFromIdeation({
        projectPath: `${projectsRootDir}/${draft.name.trim()}`,
        name: draft.name,
        author: draft.author,
        consensusSnapshot: inspiration.consensus,
        createHandoff,
        sessionId: inspiration.sessionId,
      })

      if (runIdRef.current !== runId) return

      addToast({
        title: translations.home.createSuccess,
        description: `${translations.home.projectCreatedMsg}${draft.name.trim()}`,
        variant: 'success',
      })
      await enterCreatedProject(outcome)
    } catch (error) {
      if (runIdRef.current !== runId) return

      const summary = summarizeCreateProjectError(error)
      const errorMessageMap = {
        MissingMinimumConsensus: cp.createErrorMissingMinimumConsensus,
        CoreBundleGenerationFailed: cp.createErrorCoreBundleGenerationFailed,
        PersistenceFailed: cp.createErrorPersistenceFailed,
        PlanningProviderConfigurationInvalid: cp.createErrorPlanningProviderConfigurationInvalid,
        create_project_from_ideation_unavailable: cp.createErrorCommandUnavailable,
      } as const

      addToast({
        title: translations.common.error,
        description: summary.code ? errorMessageMap[summary.code] : summary.message,
        variant: 'destructive',
      })

      if (summary.code === 'PlanningProviderConfigurationInvalid') {
        input.onOpenSettings()
      }

      setStage('launch_sheet')
    } finally {
      if (runIdRef.current === runId) {
        setSubmitting(false)
      }
    }
  }, [addToast, cp, draft, enterCreatedProject, inspiration, input, projectsRootDir, translations])

  const handleGenerateVariants = useCallback(async () => {
    const generated = await inspiration.generateVariants()
    if (generated) {
      prepareLaunchSheet(generated.variants[0]?.create_handoff)
    }
  }, [inspiration, prepareLaunchSheet])

  const handleSkipToCreateForm = useCallback(() => {
    prepareLaunchSheet()
  }, [prepareLaunchSheet])

  const handleBackToInspiration = useCallback(() => {
    setStage('ideation')
  }, [])

  const handleContinueToCreateForm = useCallback(() => {
    prepareLaunchSheet(inspiration.finalCreateHandoffDraft)
  }, [inspiration.finalCreateHandoffDraft, prepareLaunchSheet])

  const handleBackFromCreateForm = useCallback(() => {
    setStage('ideation')
  }, [])

  const handleClose = useCallback(() => {
    if (submitting) return
    resetWorkflow({ suppressInspirationAutoCreate: true })
    input.onClose()
  }, [input, resetWorkflow, submitting])

  return useMemo(() => ({
    draft,
    errors,
    stage,
    submitting,
    preserveInspirationSession,
    setPreserveInspirationSession,
    projectGenres,
    inspiration,
    updateDraft,
    toggleGenre,
    handleSubmit,
    handleGenerateVariants,
    handleSkipToCreateForm,
    handleBackToInspiration,
    handleContinueToCreateForm,
    handleBackFromCreateForm,
    handleClose,
    resetWorkflow,
  }), [
    draft,
    errors,
    handleClose,
    handleContinueToCreateForm,
    handleGenerateVariants,
    handleBackFromCreateForm,
    handleBackToInspiration,
    handleSkipToCreateForm,
    handleSubmit,
    inspiration,
    preserveInspirationSession,
    projectGenres,
    resetWorkflow,
    stage,
    submitting,
    toggleGenre,
    updateDraft,
  ])
}
