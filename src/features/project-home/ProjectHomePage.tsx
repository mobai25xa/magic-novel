import { useEffect, useMemo, useState } from 'react'
import { ArrowRight, CheckCircle2, ClipboardCheck, FileText, PlayCircle, RefreshCw } from 'lucide-react'

import { openEditorTarget } from '@/features/editor-navigation/open-editor-target'
import {
  getPlanningManifestEntry,
  refreshPlanningManifestEntry,
  updatePlanningDocumentApprovalStateEntry,
  type PlanningDocApprovalState,
  type PlanningManifest,
} from '@/features/project-home'
import { useTranslation } from '@/hooks/use-translation'
import { Badge, Button, Spinner, useToast } from '@/magic-ui/components'
import { useNavigationStore } from '@/stores/navigation-store'
import { useProjectStore } from '@/stores/project-store'

import {
  isPlanningDocConfirmed,
  isPlanningDocReady,
  resolveBundleTone,
  resolvePlanningDocDisplayName,
  resolvePlanningEntryDisplayName,
  resolvePrimaryContractTarget,
  resolveRecommendedPlanningTarget,
} from './planning-manifest-helpers'

function formatUpdatedAt(timestamp?: number) {
  if (!timestamp) {
    return 'N/A'
  }

  return new Date(timestamp).toLocaleString()
}

function BundleStatusBadge(input: { bundleStatus: string; label: string }) {
  return <Badge color={resolveBundleTone(input.bundleStatus)}>{input.label}</Badge>
}

function DocBadges(input: { doc: PlanningManifest['docs'][number]; tr: ReturnType<typeof useTranslation>['translations']['projectHome'] }) {
  return (
    <div className="flex flex-wrap gap-2">
      <Badge color={isPlanningDocReady(input.doc) ? 'success' : 'warning'}>
        {isPlanningDocReady(input.doc) ? input.tr.materializationReady : input.tr.materializationMissing}
      </Badge>
      <Badge color={input.doc.approval_state === 'accepted' ? 'success' : input.doc.approval_state === 'user_refined' ? 'info' : 'warning'}>
        {input.doc.approval_state === 'accepted'
          ? input.tr.approvalAccepted
          : input.doc.approval_state === 'user_refined'
            ? input.tr.approvalUserRefined
            : input.tr.approvalAiDraft}
      </Badge>
      {input.doc.required_for_create ? <Badge variant="outline">{input.tr.requiredForCreate}</Badge> : null}
      {input.doc.required_for_write ? <Badge variant="outline">{input.tr.requiredForWrite}</Badge> : null}
    </div>
  )
}

function resolveBundleLabel(
  bundleStatus: string,
  canStart: boolean,
  tr: ReturnType<typeof useTranslation>['translations']['projectHome'],
) {
  if (canStart) {
    return tr.bundleReadyForWrite
  }

  if (bundleStatus === 'ready') {
    return tr.bundleContractsReady
  }

  if (bundleStatus === 'failed') {
    return tr.bundleMissingCoreDocs
  }

  if (bundleStatus === 'ready_for_write') {
    return tr.bundleReadyForWrite
  }

  if (bundleStatus === 'missing_core_docs') {
    return tr.bundleMissingCoreDocs
  }

  return tr.bundlePlanningInProgress
}

export function ProjectHomePage() {
  const { translations } = useTranslation()
  const tr = translations.projectHome
  const { addToast } = useToast()
  const navigate = useNavigationStore((state) => state.navigate)
  const {
    project,
    projectPath,
    planningManifest,
    planningManifestProjectPath,
    setPlanningManifest,
  } = useProjectStore()

  const [loadingManifest, setLoadingManifest] = useState(false)
  const [updatingDocId, setUpdatingDocId] = useState<string | null>(null)

  const activeManifest = projectPath && planningManifestProjectPath === projectPath
    ? planningManifest
    : null

  useEffect(() => {
    if (!projectPath || activeManifest || loadingManifest) {
      return
    }

    let cancelled = false
    setLoadingManifest(true)

    void getPlanningManifestEntry(projectPath)
      .then((manifest) => {
        if (cancelled) {
          return
        }
        setPlanningManifest(projectPath, manifest)
      })
      .catch((error) => {
        if (cancelled) {
          return
        }
        console.error('[project-home] Failed to load planning manifest:', error)
        addToast({
          title: tr.manifestRefreshFailedTitle,
          description: String(error),
          variant: 'destructive',
        })
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingManifest(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [activeManifest, addToast, loadingManifest, projectPath, setPlanningManifest, tr.manifestRefreshFailedTitle])

  const blockerText = useMemo(() => {
    if (!activeManifest?.writing_readiness.blockers.length) {
      return [tr.noBlockers]
    }

    return activeManifest.writing_readiness.blockers.map((blocker) => {
      if (blocker === 'narrative_contract_unconfirmed') {
        return tr.blockerNarrativeContract
      }
      if (blocker === 'chapter_1_detail_unconfirmed') {
        return tr.blockerChapter1Detail
      }
      return blocker
    })
  }, [activeManifest?.writing_readiness.blockers, tr.blockerChapter1Detail, tr.blockerNarrativeContract, tr.noBlockers])

  const recommendedDocDisplayName = useMemo(() => {
    if (!activeManifest?.recommended_next_doc) {
      return tr.noRecommendedDoc
    }

    return resolvePlanningDocDisplayName(activeManifest.recommended_next_doc) || activeManifest.recommended_next_doc
  }, [activeManifest?.recommended_next_doc, tr.noRecommendedDoc])

  const openTargetInEditor = async (targetRef: string | null) => {
    navigate('editor')
    if (!targetRef) {
      return
    }

    const opened = await openEditorTarget(targetRef, {
      revealLeftTree: true,
      switchLeftTab: true,
    })

    if (!opened) {
      throw new Error(`Failed to open target: ${targetRef}`)
    }
  }

  const handleRefreshManifest = async () => {
    if (!projectPath) {
      return
    }

    setLoadingManifest(true)
    try {
      const manifest = await refreshPlanningManifestEntry(projectPath)
      setPlanningManifest(projectPath, manifest)
      addToast({
        title: tr.manifestRefreshSuccessTitle,
        variant: 'success',
      })
    } catch (error) {
      console.error('[project-home] Failed to refresh planning manifest:', error)
      addToast({
        title: tr.manifestRefreshFailedTitle,
        description: String(error),
        variant: 'destructive',
      })
    } finally {
      setLoadingManifest(false)
    }
  }

  const handleAcceptDoc = async (docId: string) => {
    if (!projectPath) {
      return
    }

    setUpdatingDocId(docId)
    try {
      const nextManifest = await updatePlanningDocumentApprovalStateEntry(
        projectPath,
        docId,
        'accepted' as PlanningDocApprovalState,
      )
      setPlanningManifest(projectPath, nextManifest)
      addToast({
        title: tr.acceptSuccess,
        variant: 'success',
      })
    } catch (error) {
      console.error('[project-home] Failed to accept planning doc:', error)
      addToast({
        title: tr.acceptFailed,
        description: String(error),
        variant: 'destructive',
      })
    } finally {
      setUpdatingDocId(null)
    }
  }

  const handleContinuePlanning = async () => {
    try {
      await openTargetInEditor(resolveRecommendedPlanningTarget(activeManifest))
    } catch (error) {
      addToast({
        title: tr.openDocFailed,
        description: String(error),
        variant: 'destructive',
      })
    }
  }

  const handleViewContracts = async () => {
    try {
      await openTargetInEditor(resolvePrimaryContractTarget(activeManifest))
    } catch (error) {
      addToast({
        title: tr.openDocFailed,
        description: String(error),
        variant: 'destructive',
      })
    }
  }

  const handleStartWriting = async () => {
    if (!activeManifest?.writing_readiness.can_start) {
      addToast({
        title: tr.startWritingBlockedTitle,
        description: blockerText.join(' / '),
        variant: 'warning',
      })
      await handleContinuePlanning()
      return
    }

    navigate('editor')
    addToast({
      title: tr.startWritingReadyTitle,
      description: tr.startWritingReadyDesc,
      variant: 'success',
    })
  }

  if (!projectPath || !project) {
    return (
      <div className="flex min-h-[calc(100vh-160px)] items-center justify-center px-6 py-10">
        <div className="glass-panel-strong w-full max-w-2xl rounded-[28px] p-8 text-center">
          <h1 className="text-2xl font-semibold">{tr.noProjectTitle}</h1>
          <p className="mt-3 text-sm opacity-75">{tr.noProjectDesc}</p>
          <div className="mt-6 flex justify-center">
            <Button onClick={() => navigate('workspace')}>{tr.backToWorkspace}</Button>
          </div>
        </div>
      </div>
    )
  }

  if (!activeManifest || loadingManifest) {
    return (
      <div className="flex min-h-[calc(100vh-160px)] items-center justify-center px-6 py-10">
        <div className="glass-panel-strong flex w-full max-w-xl items-center justify-center gap-3 rounded-[28px] p-8">
          <Spinner />
          <span>{tr.loadingManifest}</span>
        </div>
      </div>
    )
  }

  return (
    <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-6 py-6">
      <section className="glass-panel-strong rounded-[30px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-6">
        <div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
          <div className="space-y-3">
            <div className="flex flex-wrap items-center gap-3">
              <h1 className="text-3xl font-semibold tracking-tight">{project.name}</h1>
              <BundleStatusBadge
                bundleStatus={activeManifest.bundle_status}
                label={resolveBundleLabel(
                  activeManifest.bundle_status,
                  activeManifest.writing_readiness.can_start,
                  tr,
                )}
              />
            </div>
            <p className="max-w-3xl text-sm opacity-75">{tr.subtitle}</p>
            <div className="flex flex-wrap items-center gap-3 text-xs opacity-70">
              <span>{tr.bundleVersion}: {activeManifest.bundle_version}</span>
              <span>{tr.optionalOutputs}: {activeManifest.optional_outputs.length}</span>
              <span>{tr.blockerCount}: {activeManifest.writing_readiness.blockers.length}</span>
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-3 lg:min-w-[520px]">
            <Button className="justify-between" onClick={() => void handleContinuePlanning()}>
              <span>{tr.continuePlanning}</span>
              <ArrowRight size={16} />
            </Button>
            <Button variant="outline" className="justify-between" onClick={() => void handleViewContracts()}>
              <span>{tr.viewContracts}</span>
              <FileText size={16} />
            </Button>
            <Button variant="secondary" className="justify-between" onClick={() => void handleStartWriting()}>
              <span>{tr.startWriting}</span>
              <PlayCircle size={16} />
            </Button>
          </div>
        </div>
      </section>

      <section className="grid gap-6 xl:grid-cols-[1.2fr_2fr]">
        <div className="space-y-6">
          <div className="glass-panel-strong rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-5">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="text-sm font-semibold">{tr.readinessTitle}</div>
                <p className="mt-1 text-xs opacity-70">{tr.readinessDesc}</p>
              </div>
              <Badge color={activeManifest.writing_readiness.can_start ? 'success' : 'warning'}>
                {activeManifest.writing_readiness.can_start ? tr.canStart : tr.notReady}
              </Badge>
            </div>

            <div className="mt-4 flex flex-wrap gap-2">
              {blockerText.map((blocker) => (
                <span
                  key={blocker}
                  className="rounded-full border border-[var(--border-primary)] px-3 py-1 text-xs"
                >
                  {blocker}
                </span>
              ))}
            </div>
          </div>

          <div className="glass-panel-strong rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-5">
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="text-sm font-semibold">{tr.recommendedTitle}</div>
                <p className="mt-1 text-xs opacity-70">{tr.recommendedDesc}</p>
              </div>
              <Button variant="ghost" size="sm" onClick={() => void handleRefreshManifest()} disabled={loadingManifest}>
                <RefreshCw size={14} />
              </Button>
            </div>

            <div className="mt-4 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel-secondary)] px-4 py-3">
              <div className="text-xs uppercase tracking-[0.24em] opacity-55">{tr.nextDoc}</div>
              <div className="mt-2 text-sm font-medium">{recommendedDocDisplayName}</div>
              {activeManifest.recommended_next_doc ? (
                <div className="mt-1 break-all text-xs opacity-60">
                  {activeManifest.recommended_next_doc}
                </div>
              ) : null}
            </div>
          </div>
        </div>

        <div className="glass-panel-strong rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-5">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="text-sm font-semibold">{tr.docsTitle}</div>
              <p className="mt-1 text-xs opacity-70">{tr.docsDesc}</p>
            </div>
            <Badge variant="outline">{activeManifest.docs.length} docs</Badge>
          </div>

          <div className="mt-4 grid gap-4">
            {activeManifest.docs.map((doc) => (
              <article
                key={doc.id}
                className="rounded-[24px] border border-[var(--border-primary)] bg-[var(--bg-panel-secondary)] p-4"
              >
                <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                  <div className="space-y-3">
                    <div className="flex items-center gap-2">
                      <ClipboardCheck size={16} />
                      <h2 className="text-base font-semibold">{resolvePlanningEntryDisplayName(doc)}</h2>
                    </div>
                    <div className="text-xs opacity-70">{doc.path}</div>
                    <DocBadges doc={doc} tr={tr} />
                    <div className="text-xs opacity-60">
                      {tr.lastUpdated}: {formatUpdatedAt(doc.updated_at)}
                    </div>
                  </div>

                  <div className="flex flex-wrap gap-2 lg:max-w-[240px] lg:justify-end">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => void openTargetInEditor(`knowledge:${doc.path}`)}
                    >
                      {tr.openDoc}
                    </Button>
                    {isPlanningDocReady(doc) && doc.approval_state !== 'accepted' ? (
                      <Button
                        size="sm"
                        onClick={() => void handleAcceptDoc(doc.id)}
                        disabled={updatingDocId === doc.id}
                      >
                        {updatingDocId === doc.id ? <Spinner /> : <CheckCircle2 size={14} />}
                        {tr.acceptDoc}
                      </Button>
                    ) : null}
                  </div>
                </div>

                {doc.required_for_write && !isPlanningDocConfirmed(doc) ? (
                  <div className="mt-3 rounded-2xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs">
                    {tr.writeGateHint}
                  </div>
                ) : null}
              </article>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}
