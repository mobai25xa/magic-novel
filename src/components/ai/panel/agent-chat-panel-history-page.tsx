import { useMemo, useState } from 'react'

import { ConfirmDialog } from '@/components/common/ConfirmDialog'
import { InputDialog } from '@/components/common/InputDialog'
import {
  AiPanelOverlayShell,
  Button,
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/magic-ui/components'
import type { AgentSessionMeta } from '@/lib/agent-chat/session'

import { useAiTranslations } from '../ai-hooks'

type Props = {
  open: boolean
  running: boolean
  loading: boolean
  sessionList: AgentSessionMeta[]
  currentSessionId?: string
  historyStateBySessionId: Record<string, string>
  deleteDialogOpen: boolean
  deleteSessionId?: string
  onOpenDeleteDialog: (sessionId: string) => void
  onCloseDeleteDialog: () => void
  onClose: () => void
  onResumeSession: (sessionId: string) => Promise<void>
  onRenameSession: (sessionId: string, title: string) => Promise<void>
  onDeleteSession: (sessionId: string) => Promise<void>
  onOpenMissionPanel: () => void
}

function formatSessionTitle(input: { item: AgentSessionMeta; fallback: string }) {
  const title = input.item.title?.trim()
  if (title) {
    return title
  }

  return `${input.fallback} · ${new Date(input.item.created_at).toLocaleString()}`
}

function SessionRow(input: {
  item: AgentSessionMeta
  fallbackTitle: string
  renameLabel: string
  deleteLabel: string
  stateLabel: string
  selected: boolean
  disabled: boolean
  onResume: () => void
  onRename: () => void
  onDelete: () => void
}) {
  const title = formatSessionTitle({ item: input.item, fallback: input.fallbackTitle })

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <button
          type="button"
          className={`w-full text-left rounded border px-3 py-2 transition-colors ${
            input.selected ? 'border-primary active-bg' : 'border-border hover-bg'
          }`}
          disabled={input.disabled}
          onClick={input.onResume}
          title={title}
        >
          <div className="text-sm font-medium truncate text-foreground">{title}</div>
          <div className="text-[11px] text-muted-foreground mt-0.5">
            {new Date(input.item.updated_at).toLocaleString()}
          </div>
          <div className="mt-1 inline-flex rounded border border-border px-1.5 py-0.5 text-[10px] text-muted-foreground">
            {input.stateLabel}
          </div>
        </button>
      </ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={input.onRename}>{input.renameLabel}</ContextMenuItem>
        <ContextMenuItem onClick={input.onDelete} className="text-destructive">{input.deleteLabel}</ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}

export function AgentChatPanelHistoryPage(input: Props) {
  const ai = useAiTranslations()
  const [renameTarget, setRenameTarget] = useState<AgentSessionMeta | null>(null)

  const hasSessions = input.sessionList.length > 0
  const canOperate = !input.running && !input.loading

  const sortedSessions = useMemo(
    () => [...input.sessionList].sort((a, b) => b.updated_at - a.updated_at),
    [input.sessionList],
  )

  if (!input.open) {
    return null
  }

  return (
    <AiPanelOverlayShell className="flex flex-col">
      <div className="h-11 px-3 border-b flex items-center justify-between">
        <h3 className="text-sm font-semibold">{ai.panel.historyPageTitle}</h3>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            onClick={input.onOpenMissionPanel}
          >
            Mission
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 px-2 text-xs"
            onClick={input.onClose}
          >
            {ai.action.closeHistoryPage}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-3 space-y-2">
        {!hasSessions ? (
          <div className="text-xs text-muted-foreground">{ai.panel.historyEmpty}</div>
        ) : (
          sortedSessions.map((item) => (
            <SessionRow
              key={item.session_id}
              item={item}
              fallbackTitle={ai.panel.history}
              renameLabel={ai.action.renameSession}
              deleteLabel={ai.action.deleteSession}
              stateLabel={input.historyStateBySessionId[item.session_id] || ai.panel.historyStateReadOnly}
              selected={item.session_id === input.currentSessionId}
              disabled={!canOperate}
              onResume={() => {
                void input.onResumeSession(item.session_id).then(() => {
                  input.onClose()
                }).catch(() => {
                })
              }}
              onRename={() => setRenameTarget(item)}
              onDelete={() => {
                input.onOpenDeleteDialog(item.session_id)
              }}
            />
          ))
        )}
      </div>

      <InputDialog
        open={Boolean(renameTarget)}
        title={ai.panel.renameSessionTitle}
        placeholder={ai.panel.renameSessionPlaceholder}
        defaultValue={renameTarget?.title || ''}
        onClose={() => setRenameTarget(null)}
        onConfirm={(value) => {
          if (!renameTarget) {
            return
          }
          void input.onRenameSession(renameTarget.session_id, value)
          setRenameTarget(null)
        }}
      />

      <ConfirmDialog
        open={input.deleteDialogOpen}
        title={ai.action.deleteSession}
        description={ai.panel.historyDeleteConfirm}
        danger
        onCancel={input.onCloseDeleteDialog}
        onConfirm={() => {
          if (!input.deleteSessionId) {
            return
          }
          void input.onDeleteSession(input.deleteSessionId).then(() => {
            if (input.deleteSessionId === input.currentSessionId) {
              input.onClose()
            }
            input.onCloseDeleteDialog()
          }).catch(() => {
            input.onCloseDeleteDialog()
          })
        }}
      />
    </AiPanelOverlayShell>
  )
}
