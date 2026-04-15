import { useEffect } from 'react'

import { useNavigationStore } from '@/stores/navigation-store'
import { cn } from '@/lib/utils'

import { CreateProjectGeneratingPanel } from './CreateProjectGeneratingPanel'
import { CreateProjectLaunchSheet } from './CreateProjectLaunchSheet'
import { InspirationWorkspace } from './inspiration/InspirationWorkspace'
import type { CreatePageProps } from './types'
import { useCreateProjectWorkflow } from './use-create-project-workflow'

export function CreatePage({ onCreated }: CreatePageProps) {
  const navigate = useNavigationStore((state) => state.navigate)

  const workflow = useCreateProjectWorkflow({
    active: true,
    onOpenSettings: () => navigate('settings'),
    onClose: () => navigate('workspace'),
    onProjectReady: onCreated,
  })

  const isWorkspaceStage = workflow.stage === 'ideation'

  useEffect(() => {
    const mainScroll = document.querySelector('.main-scroll')
    if (!mainScroll) {
      return
    }

    mainScroll.classList.toggle('main-scroll--workspace-mode', isWorkspaceStage)

    return () => {
      mainScroll.classList.remove('main-scroll--workspace-mode')
    }
  }, [isWorkspaceStage])

  return (
    <div className={cn('content-scroll-create', isWorkspaceStage && 'content-scroll-create--workspace')}>
      <div
        className={cn(
          'creation-container',
          isWorkspaceStage && 'creation-container--workspace',
          isWorkspaceStage && 'creation-container--fixed-workspace',
        )}
      >
        <div
          className={cn(
            'glass-panel-strong create-glass-card space-y-6',
            isWorkspaceStage && 'create-glass-card--workspace',
            isWorkspaceStage && 'create-glass-card--fixed-workspace',
          )}
        >
          {workflow.stage === 'ideation' ? (
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

          {workflow.stage === 'launch_sheet' ? (
            <CreateProjectLaunchSheet
              draft={workflow.draft}
              errors={workflow.errors}
              createHandoff={workflow.inspiration.finalCreateHandoffDraft ?? null}
              submitting={workflow.submitting}
              onChange={workflow.updateDraft}
              onBack={workflow.handleBackFromCreateForm}
              onSubmit={() => {
                void workflow.handleSubmit()
              }}
            />
          ) : null}

          {workflow.stage === 'generating_contract' ? (
            <CreateProjectGeneratingPanel projectName={workflow.draft.name} />
          ) : null}
        </div>

        {!isWorkspaceStage ? <div style={{ height: 40 }} /> : null}
      </div>
    </div>
  )
}
