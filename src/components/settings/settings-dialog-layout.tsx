import type { ReactNode } from 'react'

import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle, Tabs, Tab } from '@/magic-ui/components'

import { SettingsButton } from './settings-dialog-button'
import type { SettingsTabId } from './settings-dialog-types'

type SettingsTabItem = {
  id: SettingsTabId
  label: string
  icon: ReactNode
}

export function SettingsDialogLayout(input: {
  open: boolean
  title: string
  tabs: SettingsTabItem[]
  activeTab: SettingsTabId
  setActiveTab: (id: SettingsTabId) => void
  content: ReactNode
  cancelLabel: string
  saveLabel: string
  onCancel: () => void
  onSave: () => void
}) {
  return (
    <Modal open={input.open} onOpenChange={(isOpen) => !isOpen && input.onCancel()}>
      <ModalContent size="lg" className="max-w-[750px] h-[500px] p-0 gap-0 flex flex-col">
        <ModalHeader className="px-6 py-4 shrink-0" style={{ borderBottom: "1px solid var(--border-color)" }}>
          <ModalTitle>{input.title}</ModalTitle>
          <ModalDescription className="sr-only">{input.title}</ModalDescription>
        </ModalHeader>

        <div className="flex flex-1 overflow-hidden">
          <div className="w-48 settings-panel shrink-0">
            <Tabs
              value={input.activeTab}
              onValueChange={(v) => input.setActiveTab(v as SettingsTabId)}
              orientation="vertical"
              className="p-3 space-y-1"
            >
              {input.tabs.map((tab) => (
                <Tab
                  key={tab.id}
                  value={tab.id}
                  className={`settings-nav-item w-full gap-2 text-sm`}
                >
                  <span className="settings-nav-icon">{tab.icon}</span>
                  <span className="settings-nav-label">{tab.label}</span>
                </Tab>
              ))}
            </Tabs>
          </div>

          <div className="flex-1 flex flex-col overflow-hidden">
            <div className="flex-1 overflow-y-auto p-6">{input.content}</div>

            <div className="flex justify-end gap-3 px-6 py-4 shrink-0" style={{ borderTop: "1px solid var(--border-color)" }}>
              <SettingsButton variant="outline" onClick={input.onCancel}>{input.cancelLabel}</SettingsButton>
              <SettingsButton onClick={input.onSave}>
                {input.saveLabel}
              </SettingsButton>
            </div>
          </div>
        </div>
      </ModalContent>
    </Modal>
  )
}
