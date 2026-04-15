import { useEffect, useMemo, useState } from 'react'
import { Clock3, PencilLine, Plus, RefreshCw, Trash2 } from 'lucide-react'

import { ConfirmDialog } from '@/components/common/ConfirmDialog'
import { useTranslation } from '@/hooks/use-translation'
import {
  type InspirationSessionMeta,
} from '@/features/inspiration'
import {
  Badge,
  Button,
  Input,
  Modal,
  ModalContent,
  ModalDescription,
  ModalHeader,
  ModalTitle,
  Spinner,
  Toggle,
} from '@/magic-ui/components'
import { cn } from '@/lib/utils'

interface InspirationSessionPanelProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  sessionId: string | null
  sessionList: InspirationSessionMeta[]
  loadingSession: boolean
  loadingSessionList: boolean
  sessionListError: string | null
  runningTurn: boolean
  preserveInspirationSession: boolean
  setPreserveInspirationSession: (value: boolean) => void
  loadSessionList: (limit?: number) => Promise<void>
  openSession: (sessionId: string) => Promise<void>
  newSession: () => Promise<void>
  renameSession: (sessionId: string, title: string) => Promise<void>
  deleteSession: (sessionId: string) => Promise<void>
}

function formatSessionTitle(item: InspirationSessionMeta | null, untitledLabel: string) {
  const title = item?.title?.trim()
  return title || untitledLabel
}

function formatSessionTime(timestamp?: number) {
  if (!timestamp) {
    return '...'
  }

  return new Date(timestamp).toLocaleString()
}

function formatSessionDigest(item: InspirationSessionMeta | null) {
  if (!item) {
    return null
  }

  const bits = [
    typeof item.last_turn === 'number' ? `#${item.last_turn}` : null,
    item.last_stop_reason?.trim() || null,
  ].filter((value): value is string => Boolean(value))

  return bits.length > 0 ? bits.join(' · ') : null
}

function RenameSessionDialog(input: {
  defaultValue: string
  title: string
  onClose: () => void
  onConfirm: (value: string) => void
}) {
  const { translations } = useTranslation()
  const [value, setValue] = useState(input.defaultValue)

  return (
    <Modal open onOpenChange={(open) => !open && input.onClose()}>
      <ModalContent size="sm">
        <ModalHeader>
          <ModalTitle>{input.title}</ModalTitle>
          <ModalDescription className="sr-only">{input.title}</ModalDescription>
        </ModalHeader>
        <div className="space-y-4 p-6">
          <Input
            value={value}
            onChange={(event) => setValue(event.target.value)}
            placeholder={translations.createPage.inspirationSessionUntitled}
            autoFocus
          />
          <div className="flex justify-end gap-2">
            <Button variant="secondary" onClick={input.onClose}>
              {translations.common.cancel}
            </Button>
            <Button
              onClick={() => {
                input.onConfirm(value)
                input.onClose()
              }}
            >
              {translations.common.confirm}
            </Button>
          </div>
        </div>
      </ModalContent>
    </Modal>
  )
}

function SessionRow(input: {
  item: InspirationSessionMeta
  selected: boolean
  switchDisabled: boolean
  deleteDisabled: boolean
  untitledLabel: string
  currentLabel: string
  deleteLabel: string
  onClick: () => void
  onDelete: () => void
}) {
  return (
    <div
      className={cn(
        'rounded-2xl border px-4 py-3 transition-colors',
        input.selected
          ? 'border-sky-500/35 bg-sky-500/10'
          : 'border-[var(--border-primary)] bg-[var(--bg-base)] hover:bg-[var(--bg-panel)]',
        input.switchDisabled && input.deleteDisabled && 'opacity-60',
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <button
          type="button"
          disabled={input.switchDisabled}
          onClick={input.onClick}
          className={cn(
            'min-w-0 flex-1 text-left',
            input.switchDisabled && 'cursor-not-allowed',
          )}
          title={formatSessionTitle(input.item, input.untitledLabel)}
        >
          <div className="truncate text-sm font-medium">
            {formatSessionTitle(input.item, input.untitledLabel)}
          </div>
          <div className="mt-1 flex items-center gap-1 text-[11px] opacity-65">
            <Clock3 size={12} />
            <span>{formatSessionTime(input.item.updated_at)}</span>
          </div>
        </button>
        <div className="flex shrink-0 items-center gap-2">
          {input.selected ? (
            <Badge variant="soft" color="primary">{input.currentLabel}</Badge>
          ) : null}
          <Button
            size="sm"
            variant="ghost"
            disabled={input.deleteDisabled}
            className="h-8 px-2"
            aria-label={input.deleteLabel}
            onClick={input.onDelete}
          >
            <Trash2 size={14} />
          </Button>
        </div>
      </div>
      {formatSessionDigest(input.item) ? (
        <div className="mt-2 text-xs opacity-70">{formatSessionDigest(input.item)}</div>
      ) : null}
    </div>
  )
}

export function InspirationSessionPanel(input: InspirationSessionPanelProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage
  const {
    open,
    onOpenChange,
    sessionId,
    sessionList,
    loadingSession,
    loadingSessionList,
    sessionListError,
    runningTurn,
    preserveInspirationSession,
    setPreserveInspirationSession,
    loadSessionList,
    openSession,
    newSession,
    renameSession,
    deleteSession,
  } = input
  const [renameOpen, setRenameOpen] = useState(false)
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [deleteTargetSessionId, setDeleteTargetSessionId] = useState<string | null>(null)

  const currentSession = useMemo(
    () => sessionList.find((item) => item.session_id === sessionId) ?? null,
    [sessionId, sessionList],
  )
  const deleteTargetSession = useMemo(
    () => sessionList.find((item) => item.session_id === deleteTargetSessionId) ?? null,
    [deleteTargetSessionId, sessionList],
  )

  const currentTitle = sessionId
    ? formatSessionTitle(currentSession, cp.inspirationSessionUntitled)
    : cp.inspirationSessionNoActive

  const currentTime = currentSession
    ? formatSessionTime(currentSession.updated_at)
    : loadingSession
      ? translations.common.loading
      : '...'

  const currentDigest = formatSessionDigest(currentSession)
  const actionLocked = loadingSession || loadingSessionList
  const switchLocked = actionLocked || runningTurn
  const currentDeleteDisabled = actionLocked || !sessionId || runningTurn
  const currentRenameDisabled = actionLocked || !sessionId || runningTurn
  const newSessionDisabled = actionLocked || runningTurn
  const deleteTargetTitle = formatSessionTitle(deleteTargetSession, cp.inspirationSessionUntitled)

  const closeDeleteDialog = () => {
    setDeleteOpen(false)
    setDeleteTargetSessionId(null)
  }

  const openDeleteDialog = (nextSessionId: string) => {
    setDeleteTargetSessionId(nextSessionId)
    setDeleteOpen(true)
  }

  useEffect(() => {
    if (open) {
      void loadSessionList(20)
    }
  }, [loadSessionList, open])

  return (
    <>
      <Modal open={open} onOpenChange={onOpenChange}>
        <ModalContent size="lg" className="max-h-[88vh] overflow-hidden">
          <ModalHeader className="border-b border-[var(--border-primary)] px-6 py-5">
            <div className="flex items-center justify-between gap-3">
              <div>
                <ModalTitle>{cp.inspirationSessionPanelTitle}</ModalTitle>
                <ModalDescription className="mt-1 text-sm opacity-70">
                  {cp.inspirationSessionListLabel}
                </ModalDescription>
              </div>
              <Button
                size="sm"
                variant="outline"
                disabled={loadingSessionList}
                onClick={() => {
                  void loadSessionList(20)
                }}
              >
                {loadingSessionList ? (
                  <Spinner size="xs" className="mr-1.5" />
                ) : (
                  <RefreshCw size={14} className="mr-1.5" />
                )}
                {translations.common.retry}
              </Button>
            </div>
          </ModalHeader>

          <div className="grid gap-4 overflow-y-auto p-6 lg:grid-cols-[minmax(0,320px)_minmax(0,1fr)]">
            <div className="space-y-4">
              <section className="rounded-[24px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
                <div className="mb-3 text-xs font-semibold uppercase tracking-[0.18em] opacity-60">
                  {cp.inspirationSessionCurrentLabel}
                </div>
                <div className="space-y-3">
                  <div>
                    <div className="text-base font-semibold">{currentTitle}</div>
                    <div className="mt-1 flex items-center gap-1 text-xs opacity-70">
                      <Clock3 size={12} />
                      <span>{currentTime}</span>
                    </div>
                  </div>
                  {currentDigest ? (
                    <div className="rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-base)] px-3 py-2 text-xs opacity-75">
                      {currentDigest}
                    </div>
                  ) : null}
                  {!input.sessionId ? (
                    <div className="rounded-2xl border border-dashed border-[var(--border-primary)] px-3 py-3 text-xs opacity-70">
                      {cp.inspirationSessionNoActive}
                    </div>
                  ) : null}
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={newSessionDisabled}
                      onClick={() => {
                        void newSession().then(() => onOpenChange(false))
                      }}
                    >
                      <Plus size={14} className="mr-1.5" />
                      {cp.inspirationSessionNew}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={currentRenameDisabled}
                      onClick={() => setRenameOpen(true)}
                    >
                      <PencilLine size={14} className="mr-1.5" />
                      {cp.inspirationSessionRename}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={currentDeleteDisabled}
                      onClick={() => {
                        if (sessionId) {
                          openDeleteDialog(sessionId)
                        }
                      }}
                    >
                      <Trash2 size={14} className="mr-1.5" />
                      {cp.inspirationSessionDelete}
                    </Button>
                  </div>
                </div>
              </section>

              <section className="rounded-[24px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="text-sm font-semibold">{cp.inspirationSessionPreserveToggle}</div>
                    <div className="mt-1 text-xs opacity-70">
                      {cp.inspirationSessionPreserveHint}
                    </div>
                  </div>
                  <Toggle
                    checked={preserveInspirationSession}
                    onChange={(event) => setPreserveInspirationSession(event.target.checked)}
                    aria-label={cp.inspirationSessionPreserveToggle}
                  />
                </div>
              </section>

              {actionLocked || runningTurn ? (
                <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-xs">
                  {actionLocked ? cp.inspirationSessionLoadingHint : cp.inspirationSessionRunningHint}
                </div>
              ) : null}

              {sessionListError ? (
                <div className="rounded-2xl border border-red-500/25 bg-red-500/10 px-4 py-3 text-xs">
                  {sessionListError}
                </div>
              ) : null}
            </div>

            <section className="min-h-0 rounded-[24px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
              <div className="mb-3 text-xs font-semibold uppercase tracking-[0.18em] opacity-60">
                {cp.inspirationSessionListLabel}
              </div>
              <div className="space-y-3">
                {loadingSessionList && sessionList.length === 0 ? (
                  <div className="flex items-center justify-center rounded-2xl border border-dashed border-[var(--border-primary)] px-4 py-10">
                    <Spinner />
                  </div>
                ) : null}

                {!loadingSessionList && sessionList.length === 0 ? (
                  <div className="rounded-2xl border border-dashed border-[var(--border-primary)] px-4 py-10 text-center text-sm opacity-70">
                    {cp.inspirationSessionEmptyList}
                  </div>
                ) : null}

                {sessionList.map((item) => (
                  <SessionRow
                    key={item.session_id}
                    item={item}
                    selected={item.session_id === sessionId}
                    switchDisabled={switchLocked}
                    deleteDisabled={switchLocked}
                    untitledLabel={cp.inspirationSessionUntitled}
                    currentLabel={cp.inspirationSessionCurrentLabel}
                    deleteLabel={cp.inspirationSessionDelete}
                    onClick={() => {
                      void openSession(item.session_id).then(() => onOpenChange(false))
                    }}
                    onDelete={() => {
                      openDeleteDialog(item.session_id)
                    }}
                  />
                ))}
              </div>
            </section>
          </div>
        </ModalContent>
      </Modal>

      {renameOpen ? (
        <RenameSessionDialog
          title={cp.inspirationSessionRename}
          defaultValue={currentSession?.title ?? ''}
          onClose={() => setRenameOpen(false)}
          onConfirm={(value) => {
            if (!sessionId) {
              return
            }
            void renameSession(sessionId, value)
          }}
        />
      ) : null}

      <ConfirmDialog
        open={deleteOpen}
        title={cp.inspirationSessionDeleteConfirmTitle}
        description={`${cp.inspirationSessionDeleteConfirmDesc}\n${deleteTargetTitle}`}
        danger
        onCancel={closeDeleteDialog}
        onConfirm={() => {
          if (!deleteTargetSessionId) {
            return
          }
          void deleteSession(deleteTargetSessionId)
          closeDeleteDialog()
        }}
      />
    </>
  )
}
