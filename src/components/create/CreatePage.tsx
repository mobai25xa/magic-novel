import { useEffect } from 'react'

import { Button } from '@/magic-ui/components'
import { useNavigationStore } from '@/stores/navigation-store'
import { useTranslation } from '@/hooks/use-translation'
import { cn } from '@/lib/utils'

import { CreateProjectBootstrapPanel } from './CreateProjectBootstrapPanel'
import { CreateProjectForm } from './CreateProjectForm'
import { CreateProjectResultPanel } from './CreateProjectResultPanel'
import { InspirationVariantReview } from './inspiration/InspirationVariantReview'
import { InspirationWorkspace } from './inspiration/InspirationWorkspace'
import type { CreatePageProps } from './types'
import { useCreateProjectWorkflow } from './use-create-project-workflow'

export function CreatePage({ onCreated }: CreatePageProps) {
  const { translations } = useTranslation()
  const navigate = useNavigationStore((state) => state.navigate)

  const workflow = useCreateProjectWorkflow({
    active: true,
    onOpenSettings: () => navigate('settings'),
    onClose: () => navigate('workspace'),
    onProjectReady: onCreated,
  })
  const isWorkspaceStage =
    workflow.stage === 'inspiration_chat' || workflow.stage === 'variant_review'
  const isFixedWorkspaceStage = workflow.stage === 'inspiration_chat'

  useEffect(() => {
    const mainScroll = document.querySelector('.main-scroll')
    if (!mainScroll) {
      return
    }

    mainScroll.classList.toggle('main-scroll--workspace-mode', isFixedWorkspaceStage)

    return () => {
      mainScroll.classList.remove('main-scroll--workspace-mode')
    }
  }, [isFixedWorkspaceStage])

  return (
    <div className={cn('content-scroll-create', isFixedWorkspaceStage && 'content-scroll-create--workspace')}>
      <div
        className={cn(
          'creation-container',
          isWorkspaceStage && 'creation-container--workspace',
          isFixedWorkspaceStage && 'creation-container--fixed-workspace',
        )}
      >
        {!isWorkspaceStage ? (
          <div className="create-page-header">
            <h1 className="create-page-title">{translations.createPage.pageTitle}</h1>
            <p className="create-page-subtitle">{translations.createPage.pageSubtitle}</p>
          </div>
        ) : null}

        <div
          className={cn(
            'glass-panel-strong create-glass-card space-y-6',
            isWorkspaceStage && 'create-glass-card--workspace',
            isFixedWorkspaceStage && 'create-glass-card--fixed-workspace',
          )}
        >
          {workflow.stage === 'inspiration_chat' ? (
            <InspirationWorkspace
              data={workflow.inspiration}
              preserveInspirationSession={workflow.preserveInspirationSession}
              setPreserveInspirationSession={workflow.setPreserveInspirationSession}
              onGenerateVariants={() => {
                void workflow.handleGenerateVariants()
              }}
              onSkipToCreateForm={workflow.handleSkipToCreateForm}
            />
          ) : null}

          {workflow.stage === 'variant_review' ? (
            <InspirationVariantReview
              data={workflow.inspiration}
              onBack={workflow.handleBackToInspiration}
              onContinue={workflow.handleContinueToCreateForm}
              onRegenerate={() => {
                void workflow.handleGenerateVariants()
              }}
            />
          ) : null}

          {workflow.stage === 'create_form' ? (
            <div className="space-y-4">
              <div className="flex flex-wrap justify-between gap-3 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] px-4 py-3">
                <div>
                  <div className="text-sm font-semibold">{translations.createPage.inspirationContinueToForm}</div>
                  <p className="mt-1 text-xs opacity-70">{translations.createPage.inspirationFormHint}</p>
                </div>
                <Button variant="outline" onClick={workflow.handleBackFromCreateForm} disabled={workflow.submitting}>
                  {translations.createPage.inspirationBackToWorkspaceFlow}
                </Button>
              </div>

              <CreateProjectForm
                mode="page"
                draft={workflow.draft}
                errors={workflow.errors}
                projectGenres={workflow.projectGenres}
                submitting={workflow.submitting}
                onChange={workflow.updateDraft}
                onToggleGenre={workflow.toggleGenre}
                onCancel={workflow.handleClose}
                onSubmit={() => {
                  void workflow.handleSubmit()
                }}
              />
            </div>
          ) : null}

          {workflow.stage === 'progress' ? (
            <CreateProjectBootstrapPanel
              projectName={workflow.draft.name || translations.createPage.pageTitle}
              status={workflow.bootstrapStatus}
              onCancel={workflow.handleClose}
            />
          ) : null}

          {workflow.stage === 'result' && workflow.result ? (
            <CreateProjectResultPanel
              mode="page"
              result={workflow.result}
              onEnterProject={() => {
                void workflow.handleEnterProject()
              }}
              onRetryBootstrap={() => {
                void workflow.handleRetryBootstrap()
              }}
              onCreateAnother={workflow.resetWorkflow}
              onClose={workflow.handleClose}
            />
          ) : null}
        </div>

        {!isFixedWorkspaceStage ? <div style={{ height: 40 }} /> : null}
      </div>
    </div>
  )
}
