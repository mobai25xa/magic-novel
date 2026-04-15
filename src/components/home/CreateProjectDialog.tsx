import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'
import { useNavigationStore } from '@/stores/navigation-store'

import { CreateProjectGeneratingPanel } from '@/components/create/CreateProjectGeneratingPanel'
import { CreateProjectLaunchSheet } from '@/components/create/CreateProjectLaunchSheet'
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
      navigate('project_home')
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
            <CreateProjectGeneratingPanel
              projectName={workflow.draft.name || translations.home.createProject}
            />
          ) : null}
        </div>
      </ModalContent>
    </Modal>
  )
}
