import { Button, Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'
import { useNavigationStore } from '@/stores/navigation-store'

import { CreateProjectBootstrapPanel } from '@/components/create/CreateProjectBootstrapPanel'
import { CreateProjectForm } from '@/components/create/CreateProjectForm'
import { CreateProjectResultPanel } from '@/components/create/CreateProjectResultPanel'
import { InspirationVariantReview } from '@/components/create/inspiration/InspirationVariantReview'
import { InspirationWorkspace } from '@/components/create/inspiration/InspirationWorkspace'
import { useCreateProjectWorkflow } from '@/components/create/use-create-project-workflow'

interface CreateProjectDialogProps {
  open: boolean
  onClose: () => void
}

export function CreateProjectDialog({ open, onClose }: CreateProjectDialogProps) {
  const { translations } = useTranslation()
  const navigate = useNavigationStore((state) => state.navigate)

  const workflow = useCreateProjectWorkflow({
    active: open,
    onOpenSettings: () => navigate('settings'),
    onClose,
    onProjectReady: () => {
      navigate('editor')
      onClose()
    },
  })

  return (
    <Modal open={open} onOpenChange={(isOpen) => !isOpen && workflow.handleClose()}>
      <ModalContent size="xl">
        <ModalHeader>
          <ModalTitle>{translations.home.createProject}</ModalTitle>
          <ModalDescription>{translations.createPage.pageSubtitle}</ModalDescription>
        </ModalHeader>

        <div className="max-h-[80vh] overflow-y-auto p-6">
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
                mode="dialog"
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
              projectName={workflow.draft.name || translations.home.createProject}
              status={workflow.bootstrapStatus}
              onCancel={workflow.handleClose}
            />
          ) : null}

          {workflow.stage === 'result' && workflow.result ? (
            <CreateProjectResultPanel
              mode="dialog"
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
      </ModalContent>
    </Modal>
  )
}
