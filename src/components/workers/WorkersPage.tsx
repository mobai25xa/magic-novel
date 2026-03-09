import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  Route, Globe2, Users, Feather,
  Globe, Network,
  Database, Regex, UserCheck,
  SpellCheck, BookOpenCheck,
  MessageSquarePlus, Play,
} from 'lucide-react'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'

import {
  deleteWorkerFeature,
  listWorkersFeature,
  saveWorkerFeature,
  type WorkerDefinition,
} from '@/features/global-config'
import {
  Modal,
  ModalContent,
  ModalDescription,
  ModalFooter,
  ModalHeader,
  ModalTitle,
  Button,
  Input,
  Textarea,
  toast,
} from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

import { WorkerHero } from './WorkerHero'
import { TerminalCard } from './TerminalCard'
import { WorkerCard } from './WorkerCard'
import type { WorkerCardData } from './types'
import {
  AVAILABLE_WORKER_TOOLS,
  createEmptyWorkerForm,
  parseWorkerFormValue,
  safeParseWorkerJson,
  toggleTool,
  validateWorkerForm,
  workerToFormValue,
  type WorkerFormValue,
} from './worker-management'

type WorkerFormState = WorkerFormValue

const WORKER_CARD_META: Record<string, Pick<WorkerCardData, 'icon' | 'colorClass' | 'status' | 'statusLabel' | 'primaryAction' | 'primaryActionIcon'>> = {
  'plot-generator': {
    icon: Route,
    colorClass: 'bg-plot',
    status: 'running',
    statusLabel: '执行中',
    primaryAction: '交互',
    primaryActionIcon: MessageSquarePlus,
  },
  'world-builder': {
    icon: Globe2,
    colorClass: 'bg-world',
    status: 'idle',
    statusLabel: '空闲 (常驻)',
    primaryAction: '唤醒扫描',
    primaryActionIcon: Play,
  },
  'psychology-profiler': {
    icon: Users,
    colorClass: 'bg-char',
    status: 'standby',
    statusLabel: '待命',
    primaryAction: '发起推演',
    primaryActionIcon: Play,
  },
  proofreader: {
    icon: Feather,
    colorClass: 'bg-editor',
    status: 'standby',
    statusLabel: '待命',
    primaryAction: '全文扫描',
    primaryActionIcon: Play,
  },
}

const CARD_META_FALLBACK: Pick<WorkerCardData, 'icon' | 'colorClass' | 'status' | 'statusLabel' | 'primaryAction' | 'primaryActionIcon'> = {
  icon: Globe,
  colorClass: 'bg-world',
  status: 'idle',
  statusLabel: '空闲',
  primaryAction: '交互',
  primaryActionIcon: MessageSquarePlus,
}

const TOOL_ICON_BY_NAME = {
  read: BookOpenCheck,
  edit: SpellCheck,
  create: MessageSquarePlus,
  ls: Database,
  grep: Regex,
  outline: Route,
  character_sheet: UserCheck,
  search_knowledge: Network,
} as const

export function WorkersPage() {
  const { translations } = useTranslation()
  const wp = translations.workersPage

  const [workers, setWorkers] = useState<WorkerDefinition[]>([])
  const [loading, setLoading] = useState(true)
  const [managingOpen, setManagingOpen] = useState(false)
  const [selectedWorkerName, setSelectedWorkerName] = useState<string | null>(null)
  const [form, setForm] = useState<WorkerFormState>(() => createEmptyWorkerForm())

  const loadWorkers = useCallback(async () => {
    setLoading(true)
    try {
      const defs = await listWorkersFeature()
      setWorkers(defs)
      if (defs.length > 0 && !selectedWorkerName) {
        setSelectedWorkerName(defs[0].name)
      }
    } catch (error) {
      toast.error(wp.operationFailed, String(error))
    } finally {
      setLoading(false)
    }
  }, [selectedWorkerName, wp.operationFailed])

  useEffect(() => {
    void loadWorkers()
  }, [loadWorkers])

  const selectedWorker = useMemo(
    () => workers.find((worker) => worker.name === selectedWorkerName) ?? null,
    [workers, selectedWorkerName],
  )

  useEffect(() => {
    if (selectedWorker) {
      setForm(workerToFormValue(selectedWorker))
    } else {
      setForm(createEmptyWorkerForm())
    }
  }, [selectedWorker])

  const workerCards = useMemo(() => workers.map((worker, index) => toWorkerCardData(worker, index)), [workers])

  const openCreate = useCallback(() => {
    setManagingOpen(true)
    setSelectedWorkerName(null)
    setForm(createEmptyWorkerForm())
  }, [])

  const openManage = useCallback(() => {
    setManagingOpen(true)
  }, [])

  const handleSelectWorker = useCallback((name: string) => {
    setSelectedWorkerName(name)
  }, [])

  const handleSaveWorker = useCallback(async () => {
    const validationError = validateWorkerForm(form, {
      requiredName: wp.requiredName,
      requiredDisplayName: wp.requiredDisplayName,
      requiredPrompt: wp.requiredPrompt,
      requiredTools: wp.requiredTools,
    })
    if (validationError) {
      toast.warning(wp.saveFailed, validationError)
      return
    }

    try {
      const payload = parseWorkerFormValue(form)
      await saveWorkerFeature(payload)
      await loadWorkers()
      setSelectedWorkerName(payload.name)
      toast.success(selectedWorker ? wp.update : wp.saved)
    } catch (error) {
      toast.error(wp.saveFailed, String(error))
    }
  }, [form, loadWorkers, selectedWorker, wp.requiredDisplayName, wp.requiredName, wp.requiredPrompt, wp.requiredTools, wp.saveFailed, wp.saved, wp.update])

  const handleDeleteWorker = useCallback(async (name: string) => {
    if (!confirm(wp.deleteConfirm)) {
      return
    }

    try {
      await deleteWorkerFeature(name)
      await loadWorkers()
      setSelectedWorkerName((current) => (current === name ? null : current))
      toast.success(wp.deleted)
    } catch (error) {
      toast.error(wp.operationFailed, String(error))
    }
  }, [loadWorkers, wp.deleteConfirm, wp.deleted, wp.operationFailed])

  const handleImportWorker = useCallback(async () => {
    try {
      const selected = await openDialog({
        title: wp.import,
        multiple: false,
        directory: false,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      })
      if (!selected || typeof selected !== 'string') return

      const content = await readTextFile(selected)
      const parsed = safeParseWorkerJson(content)
      if (!parsed) {
        toast.warning(wp.operationFailed, wp.invalidJson)
        return
      }

      await saveWorkerFeature(parsed)
      await loadWorkers()
      setSelectedWorkerName(parsed.name)
      setManagingOpen(true)
      toast.success(wp.imported)
    } catch (error) {
      toast.error(wp.operationFailed, String(error))
    }
  }, [loadWorkers, wp.import, wp.imported, wp.invalidJson, wp.operationFailed])

  const handleExportWorker = useCallback(async (worker: WorkerDefinition) => {
    try {
      const output = await saveDialog({
        title: wp.export,
        filters: [{ name: 'JSON', extensions: ['json'] }],
        defaultPath: `${worker.name}.json`,
      })
      if (!output || typeof output !== 'string') return

      await writeTextFile(output, JSON.stringify(worker, null, 2))
      toast.success(wp.exported, output)
    } catch (error) {
      toast.error(wp.operationFailed, String(error))
    }
  }, [wp.export, wp.exported, wp.operationFailed])

  if (loading) {
    return (
      <div className="bento-grid">
        <div className="bento-card card-hero-workers">
          <div className="workers-hero-info">
            <div className="skeleton skeleton-heading" style={{ width: '60%', height: 28 }} />
            <div className="skeleton skeleton-text" style={{ width: '90%' }} />
            <div className="skeleton skeleton-text" style={{ width: '70%' }} />
          </div>
          <div className="workers-hero-visual" />
        </div>
        <div className="bento-card card-terminal span-4">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div className="skeleton" style={{ height: 14, width: '50%', background: '#1f2335' }} />
            <div className="skeleton" style={{ height: 12, width: '80%', background: '#1f2335' }} />
            <div className="skeleton" style={{ height: 12, width: '65%', background: '#1f2335' }} />
            <div className="skeleton" style={{ height: 12, width: '75%', background: '#1f2335' }} />
          </div>
        </div>
        {[1, 2, 3, 4].map((i) => (
          <div className="bento-card worker-card span-6" key={i}>
            <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
              <div className="skeleton" style={{ width: 52, height: 52, borderRadius: 16 }} />
              <div style={{ flex: 1 }}>
                <div className="skeleton skeleton-heading" style={{ width: '50%' }} />
                <div className="skeleton skeleton-text" style={{ width: '30%' }} />
              </div>
            </div>
            <div className="skeleton skeleton-rect" style={{ height: 60 }} />
            <div className="skeleton skeleton-text" style={{ width: '40%' }} />
          </div>
        ))}
      </div>
    )
  }

  return (
    <>
      <div className="bento-grid">
        <WorkerHero
          workerCount={workers.length}
          onCreate={openCreate}
          onImport={handleImportWorker}
          onManage={openManage}
        />
        <TerminalCard />
        {workerCards.length === 0 ? (
          <div className="bento-card span-12" style={{ minHeight: 220 }}>
            <div className="text-muted-foreground">{wp.noWorkersConfigured}</div>
          </div>
        ) : (
          workerCards.map((worker) => (
            <WorkerCard
              key={worker.id}
              worker={worker}
              onConfigure={() => {
                setSelectedWorkerName(worker.id)
                setManagingOpen(true)
              }}
              onRun={() => toast.info(worker.primaryAction)}
            />
          ))
        )}
      </div>

      <Modal open={managingOpen} onOpenChange={(nextOpen) => setManagingOpen(nextOpen)}>
        <ModalContent size="xl" className="max-w-[1100px] h-[720px] p-0 gap-0 flex flex-col">
          <ModalHeader className="px-6 py-4 shrink-0" style={{ borderBottom: '1px solid var(--border-color)' }}>
            <ModalTitle>{wp.title}</ModalTitle>
            <ModalDescription>{wp.description}</ModalDescription>
          </ModalHeader>

          <div className="flex flex-1 overflow-hidden">
            <div className="w-72 settings-panel shrink-0 p-3 overflow-y-auto" style={{ borderRight: '1px solid var(--border-color)' }}>
              <div className="space-y-2">
                <Button type="button" variant="default" className="w-full" onClick={openCreate}>
                  {wp.createWorker}
                </Button>
                <Button type="button" variant="outline" className="w-full" onClick={handleImportWorker}>
                  {wp.import}
                </Button>
              </div>
              <div className="mt-4 space-y-2">
                {workers.map((worker) => (
                  <button
                    key={worker.name}
                    type="button"
                    className={`settings-nav-item w-full text-left ${selectedWorkerName === worker.name ? 'active' : ''}`}
                    onClick={() => handleSelectWorker(worker.name)}
                  >
                    <div className="font-medium truncate">{worker.display_name}</div>
                    <div className="text-xs text-muted-foreground truncate">{worker.name}</div>
                  </button>
                ))}
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-6 space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.nameLabel}</div>
                  <Input
                    value={form.name}
                    onChange={(event) => setForm((prev) => ({ ...prev, name: event.target.value }))}
                    placeholder={wp.namePlaceholder}
                  />
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.displayNameLabel}</div>
                  <Input
                    value={form.display_name}
                    onChange={(event) => setForm((prev) => ({ ...prev, display_name: event.target.value }))}
                    placeholder={wp.displayNamePlaceholder}
                  />
                </div>
              </div>

              <div>
                <div className="text-xs text-muted-foreground mb-1">{wp.systemPromptLabel}</div>
                <Textarea
                  value={form.system_prompt}
                  onChange={(event) => setForm((prev) => ({ ...prev, system_prompt: event.target.value }))}
                  placeholder={wp.systemPromptPlaceholder}
                  className="min-h-[150px]"
                />
              </div>

              <div>
                <div className="text-xs text-muted-foreground mb-2">{wp.toolWhitelistLabel}</div>
                <div className="flex flex-wrap gap-2">
                  {AVAILABLE_WORKER_TOOLS.map((tool) => {
                    const selected = form.tool_whitelist.includes(tool)
                    return (
                      <button
                        key={tool}
                        type="button"
                        className={`tag ${selected ? 'tag-info' : 'tag-hover'}`}
                        onClick={() => setForm((prev) => toggleTool(prev, tool))}
                      >
                        {tool}
                      </button>
                    )
                  })}
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.matchKeywordsLabel}</div>
                  <Input
                    value={form.match_keywords}
                    onChange={(event) => setForm((prev) => ({ ...prev, match_keywords: event.target.value }))}
                    placeholder={wp.matchKeywordsPlaceholder}
                  />
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.maxRoundsLabel}</div>
                  <Input
                    value={form.max_rounds}
                    onChange={(event) => setForm((prev) => ({ ...prev, max_rounds: event.target.value }))}
                    placeholder={wp.maxRoundsPlaceholder}
                  />
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.maxToolCallsLabel}</div>
                  <Input
                    value={form.max_tool_calls}
                    onChange={(event) => setForm((prev) => ({ ...prev, max_tool_calls: event.target.value }))}
                    placeholder={wp.maxToolCallsPlaceholder}
                  />
                </div>
              </div>

              <div>
                <div className="text-xs text-muted-foreground mb-1">{wp.modelLabel}</div>
                <Input
                  value={form.model}
                  onChange={(event) => setForm((prev) => ({ ...prev, model: event.target.value }))}
                  placeholder="gpt-4o-mini"
                />
              </div>
            </div>
          </div>

          <ModalFooter className="justify-between px-6 py-4" style={{ borderTop: '1px solid var(--border-color)' }}>
            <div className="flex gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => selectedWorker && void handleExportWorker(selectedWorker)}
                disabled={!selectedWorker}
              >
                {wp.export}
              </Button>
              <Button
                type="button"
                variant="destructive"
                onClick={() => selectedWorker && void handleDeleteWorker(selectedWorker.name)}
                disabled={!selectedWorker}
              >
                {wp.delete}
              </Button>
            </div>
            <div className="flex gap-2">
              <Button type="button" variant="outline" onClick={() => setManagingOpen(false)}>
                {wp.cancel}
              </Button>
              <Button type="button" onClick={() => void handleSaveWorker()}>
                {selectedWorker ? wp.update : wp.save}
              </Button>
            </div>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </>
  )
}

function toWorkerCardData(worker: WorkerDefinition, index: number): WorkerCardData {
  const meta = WORKER_CARD_META[worker.name] ?? fallbackMetaByIndex(index)
  return {
    id: worker.name,
    title: worker.display_name || worker.name,
    subtitle: worker.name,
    icon: meta.icon,
    colorClass: meta.colorClass,
    status: meta.status,
    statusLabel: meta.statusLabel,
    systemPrompt: worker.system_prompt,
    tools: worker.tool_whitelist.map((toolName) => ({
      name: toolName,
      icon: TOOL_ICON_BY_NAME[toolName as keyof typeof TOOL_ICON_BY_NAME] ?? Globe,
    })),
    primaryAction: meta.primaryAction,
    primaryActionIcon: meta.primaryActionIcon,
  }
}

function fallbackMetaByIndex(index: number): Pick<WorkerCardData, 'icon' | 'colorClass' | 'status' | 'statusLabel' | 'primaryAction' | 'primaryActionIcon'> {
  const cycle = [
    WORKER_CARD_META['plot-generator'],
    WORKER_CARD_META['world-builder'],
    WORKER_CARD_META['psychology-profiler'],
    WORKER_CARD_META.proofreader,
  ]
  return cycle[index % cycle.length] ?? CARD_META_FALLBACK
}
