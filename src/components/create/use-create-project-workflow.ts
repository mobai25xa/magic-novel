import { useCallback, useMemo, useRef, useState } from 'react'

import {
  createProjectFlow,
  openProjectFlow,
  resumeProjectBootstrapFlow,
  type CreateProjectFlowResult,
} from '@/components/home/page/home-page-project-actions-helpers'
import {
  DEFAULT_CREATE_PROJECT_TARGET_REF,
  resolveCreateProjectTargetRef,
  shouldAutoEnterCreatedProject,
} from '@/components/create/workflow-helpers'
import { openEditorTarget } from '@/features/editor-navigation/open-editor-target'
import type { ProjectBootstrapStatus } from '@/features/project-home'
import { useTranslation } from '@/hooks/use-translation'
import { useToast } from '@/magic-ui/components'
import { useProjectStore } from '@/stores/project-store'
import { useSettingsStore } from '@/stores/settings-store'

import { buildCreateProjectInput, createDefaultProjectDraft, validateCreateProjectDraft } from './form-utils'
import { applyCreateHandoffToDraft } from './inspiration/inspiration-helpers'
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

function isBootstrapUnsupportedError(error: unknown) {
  const message = String(error).toLowerCase()
  return (
    message.includes('resume_project_bootstrap')
    && (message.includes('not found') || message.includes('unknown') || message.includes('missing'))
  )
}

export function useCreateProjectWorkflow(input: UseCreateProjectWorkflowInput) {
  const { translations } = useTranslation()
  const { addToast } = useToast()
  const projectStore = useProjectStore()
  const projectsRootDir = useSettingsStore((state) => state.projectsRootDir)
  const projectGenres = useSettingsStore((state) => state.projectGenres)
  const inspiration = useInspirationWorkflow({ enabled: input.active ?? true })

  const runIdRef = useRef(0)

  const [draft, setDraft] = useState<CreateProjectDraft>(() => createDefaultProjectDraft())
  const [errors, setErrors] = useState<CreateProjectFormErrors>({})
  const [stage, setStage] = useState<CreateProjectWorkflowStage>('inspiration_chat')
  const [submitting, setSubmitting] = useState(false)
  const [bootstrapStatus, setBootstrapStatus] = useState<ProjectBootstrapStatus | null>(null)
  const [result, setResult] = useState<CreateProjectFlowResult | null>(null)
  const [preserveInspirationSession, setPreserveInspirationSession] = useState(false)

  const resetWorkflow = useCallback((options?: { suppressInspirationAutoCreate?: boolean }) => {
    runIdRef.current += 1
    setDraft(createDefaultProjectDraft())
    setErrors({})
    setStage('inspiration_chat')
    setSubmitting(false)
    setBootstrapStatus(null)
    setResult(null)
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
      if (patch.targetTotalWords !== undefined) delete next.targetTotalWords
      if (patch.selectedGenres !== undefined || patch.customGenres !== undefined) delete next.projectType
      return next
    })
  }, [])

  const toggleGenre = useCallback((genre: string) => {
    setDraft((current) => ({
      ...current,
      selectedGenres: current.selectedGenres.includes(genre)
        ? current.selectedGenres.filter((item) => item !== genre)
        : [...current.selectedGenres, genre],
    }))
    setErrors((current) => ({ ...current, projectType: undefined }))
  }, [])

  const enterCreatedProject = useCallback(async (outcome: CreateProjectFlowResult) => {
    if (!preserveInspirationSession && inspiration.sessionId) {
      await inspiration.deleteSession(inspiration.sessionId)
    }

    await openProjectFlow({ projectStore, selectedPath: outcome.snapshot.path })
    projectStore.setBootstrapStatus(outcome.snapshot.path, outcome.bootstrapStatus)

    const targetRefs = Array.from(new Set([
      resolveCreateProjectTargetRef(outcome),
      DEFAULT_CREATE_PROJECT_TARGET_REF,
    ].filter((value): value is string => Boolean(value))))

    let opened = false
    for (const targetRef of targetRefs) {
      opened = await openEditorTarget(targetRef, {
        revealLeftTree: true,
        switchLeftTab: true,
      })
      if (opened) {
        break
      }
    }

    if (!opened) {
      console.warn('Failed to open created-project handoff target:', targetRefs)
    }

    resetWorkflow()
    await input.onProjectReady(outcome.snapshot.path)
  }, [inspiration, input, preserveInspirationSession, projectStore, resetWorkflow])

  const handleSubmit = useCallback(async () => {
    const validationErrors = validateCreateProjectDraft(draft)
    if (Object.keys(validationErrors).length > 0) {
      setErrors(validationErrors)
      return
    }

    setSubmitting(true)
    setBootstrapStatus(null)
    setResult(null)
    setStage(draft.aiAssist ? 'progress' : 'create_form')
    const runId = runIdRef.current + 1
    runIdRef.current = runId

    try {
      const outcome = await createProjectFlow({
        onOpenSettings: input.onOpenSettings,
        projectsRootDir,
        projectStore,
        addToast,
        translations: {
          common: { error: translations.common.error },
          home: {
            configureRootDir: translations.home.configureRootDir,
            createSuccess: translations.home.createSuccess,
            projectCreatedMsg: translations.home.projectCreatedMsg,
          },
        },
        data: buildCreateProjectInput(draft),
        onBootstrapStatus: (nextStatus) => {
          if (runIdRef.current !== runId) return
          setBootstrapStatus(nextStatus)
          setStage('progress')
        },
        suppressSuccessToast: true,
      })

      if (runIdRef.current !== runId) return

      if (shouldAutoEnterCreatedProject(outcome)) {
        await enterCreatedProject(outcome)
        return
      }

      setResult(outcome)
      setBootstrapStatus(outcome.bootstrapStatus)
      setStage('result')
    } catch (error) {
      if (runIdRef.current !== runId) return

      const message = String(error)
      if (message !== translations.home.configureRootDir) {
        addToast({
          title: translations.common.error,
          description: message,
          variant: 'destructive',
        })
      }
      setStage('create_form')
    } finally {
      if (runIdRef.current === runId) {
        setSubmitting(false)
      }
    }
  }, [addToast, draft, enterCreatedProject, input.onOpenSettings, projectStore, projectsRootDir, translations])

  const handleRetryBootstrap = useCallback(async () => {
    if (!result) return

    setSubmitting(true)
    setStage('progress')
    setBootstrapStatus(result.bootstrapStatus)
    const runId = runIdRef.current + 1
    runIdRef.current = runId

    try {
      const nextStatus = await resumeProjectBootstrapFlow({
        projectPath: result.snapshot.path,
        onBootstrapStatus: (nextStatusValue) => {
          if (runIdRef.current !== runId) return
          setBootstrapStatus(nextStatusValue)
        },
      })

      if (runIdRef.current !== runId) return

      const nextResult = {
        ...result,
        bootstrapStatus: nextStatus,
        bootstrapError: null,
        bootstrapUnsupported: false,
      }

      if (shouldAutoEnterCreatedProject(nextResult)) {
        await enterCreatedProject(nextResult)
        return
      }

      setResult(nextResult)
      setBootstrapStatus(nextStatus)
      setStage('result')
    } catch (error) {
      if (runIdRef.current !== runId) return

      setResult({
        ...result,
        bootstrapError: String(error),
        bootstrapUnsupported: isBootstrapUnsupportedError(error),
      })
      setStage('result')
    } finally {
      if (runIdRef.current === runId) {
        setSubmitting(false)
      }
    }
  }, [enterCreatedProject, result])

  const handleEnterProject = useCallback(async () => {
    if (!result) return
    await enterCreatedProject(result)
  }, [enterCreatedProject, result])

  const handleGenerateVariants = useCallback(async () => {
    const generated = await inspiration.generateVariants()
    if (generated) {
      setStage('variant_review')
    }
  }, [inspiration])

  const handleSkipToCreateForm = useCallback(() => {
    setStage('create_form')
  }, [])

  const handleBackToInspiration = useCallback(() => {
    setStage('inspiration_chat')
  }, [])

  const handleContinueToCreateForm = useCallback(() => {
    if (!inspiration.finalCreateHandoffDraft) {
      return
    }

    setDraft((current) => applyCreateHandoffToDraft(
      current,
      inspiration.finalCreateHandoffDraft!,
      projectGenres,
    ))
    setStage('create_form')
  }, [inspiration.finalCreateHandoffDraft, projectGenres])

  const handleBackFromCreateForm = useCallback(() => {
    if (inspiration.variants.length > 0) {
      setStage('variant_review')
      return
    }

    setStage('inspiration_chat')
  }, [inspiration.variants.length])

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
    bootstrapStatus,
    result,
    preserveInspirationSession,
    setPreserveInspirationSession,
    projectGenres,
    inspiration,
    updateDraft,
    toggleGenre,
    handleSubmit,
    handleRetryBootstrap,
    handleEnterProject,
    handleGenerateVariants,
    handleSkipToCreateForm,
    handleBackToInspiration,
    handleContinueToCreateForm,
    handleBackFromCreateForm,
    handleClose,
    resetWorkflow,
  }), [
    bootstrapStatus,
    draft,
    errors,
    handleClose,
    handleContinueToCreateForm,
    handleEnterProject,
    handleGenerateVariants,
    handleBackFromCreateForm,
    handleBackToInspiration,
    handleSkipToCreateForm,
    handleRetryBootstrap,
    handleSubmit,
    inspiration,
    preserveInspirationSession,
    projectGenres,
    resetWorkflow,
    result,
    stage,
    submitting,
    toggleGenre,
    updateDraft,
  ])
}
