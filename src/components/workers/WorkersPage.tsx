import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  type LucideIcon,
  Route, Globe2, Users, Feather,
  Globe, Network,
  Database, Regex,
  SpellCheck, BookOpenCheck,
  MessageSquarePlus, Play,
} from 'lucide-react'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'

import {
  deleteWorkerFeature,
  isBuiltinWorkerToolName,
  listWorkersFeature,
  resolveWorkerVisibleTools,
  saveWorkerFeature,
  type BuiltinWorkerToolName,
  type WorkerDefinition,
} from '@/features/global-config'
import {
  loadAgentProviderSettings,
  missionCreateFeature,
  missionStartFeature,
} from '@/features/agent-chat'
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
import { useProjectStore } from '@/state/project'
import {
  AVAILABLE_WORKER_PRESETS,
  createEmptyWorkerForm,
  parseWorkerFormValue,
  safeParseWorkerJson,
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

const TOOL_ICON_BY_NAME: Record<BuiltinWorkerToolName, LucideIcon> = {
  workspace_map: Database,
  context_read: BookOpenCheck,
  context_search: Regex,
  knowledge_read: Globe2,
  knowledge_write: Network,
  draft_write: SpellCheck,
  structure_edit: MessageSquarePlus,
  review_check: Feather,
  skill: Users,
  todowrite: Route,
} as const

export function WorkersPage() {
  const { translations } = useTranslation()
  const wp = translations.workersPage

  const projectPath = useProjectStore((state) => state.projectPath ?? '')

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
  }, [form, loadWorkers, selectedWorker, wp.requiredDisplayName, wp.requiredName, wp.requiredPrompt, wp.saveFailed, wp.saved, wp.update])

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

  const handleRunWorker = useCallback(async (worker: WorkerCardData) => {
    if (!projectPath) {
      toast.warning(wp.operationFailed, 'No project open')
      return
    }

    try {
      const settings = await loadAgentProviderSettings()
      const created = await missionCreateFeature({
        project_path: projectPath,
        title: `Sandbox · ${worker.title}`,
        mission_text: `Sandbox mission to validate worker profile: ${worker.id}`,
        features: [{
          id: 'sandbox_1',
          status: 'pending',
          description: `Validate worker ${worker.id} can run its allowed tools (safe: ls/read only).`,
          skill: worker.id,
          preconditions: [],
          depends_on: [],
          expected_behavior: [
            'Runs safely within the project directory.',
            'Uses only the tools allowed by its capability policy.',
            'Produces a short summary and exits.',
          ],
          verification_steps: [
            'Check MissionPanel for worker tool activity and completion status.',
          ],
        }],
      })

      await missionStartFeature({
        project_path: projectPath,
        mission_id: created.mission_id,
        max_workers: 1,
        provider: 'openai-compatible',
        model: settings.openai_model,
        base_url: settings.openai_base_url,
        api_key: settings.openai_api_key,
      })

      toast.success('Mission started', created.mission_id)
    } catch (error) {
      toast.error(wp.operationFailed, String(error))
    }
  }, [projectPath, wp.operationFailed])

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
              onRun={() => void handleRunWorker(worker)}
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

              <div className="grid grid-cols-4 gap-4">
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.capabilityPresetLabel}</div>
                  <select
                    value={form.capability_preset}
                    onChange={(event) => setForm((prev) => ({ ...prev, capability_preset: event.target.value as WorkerFormState['capability_preset'] }))}
                    className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
                  >
                    {AVAILABLE_WORKER_PRESETS.map((preset) => (
                      <option key={preset} value={preset}>
                        {preset}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.modeLabel}</div>
                  <select
                    value={form.mode}
                    onChange={(event) => setForm((prev) => ({ ...prev, mode: event.target.value as WorkerFormState['mode'] }))}
                    className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
                  >
                    <option value="writing">writing</option>
                    <option value="planning">planning</option>
                  </select>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.approvalModeLabel}</div>
                  <select
                    value={form.approval_mode}
                    onChange={(event) => setForm((prev) => ({ ...prev, approval_mode: event.target.value as WorkerFormState['approval_mode'] }))}
                    className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
                  >
                    <option value="auto">auto</option>
                    <option value="confirm_writes">confirm_writes</option>
                  </select>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.clarificationModeLabel}</div>
                  <select
                    value={form.clarification_mode}
                    onChange={(event) => setForm((prev) => ({ ...prev, clarification_mode: event.target.value as WorkerFormState['clarification_mode'] }))}
                    className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
                  >
                    <option value="headless_defer">headless_defer</option>
                    <option value="interactive">interactive</option>
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <label className="settings-section flex items-center justify-between px-3 py-2">
                  <span className="text-sm">{wp.allowDelegateLabel}</span>
                  <input
                    type="checkbox"
                    checked={form.allow_delegate}
                    onChange={(event) => setForm((prev) => ({ ...prev, allow_delegate: event.target.checked }))}
                  />
                </label>
                <label className="settings-section flex items-center justify-between px-3 py-2">
                  <span className="text-sm">{wp.allowSkillActivationLabel}</span>
                  <input
                    type="checkbox"
                    checked={form.allow_skill_activation}
                    onChange={(event) => setForm((prev) => ({ ...prev, allow_skill_activation: event.target.checked }))}
                  />
                </label>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.hiddenToolsLabel}</div>
                  <Input
                    value={form.hidden_tools}
                    onChange={(event) => setForm((prev) => ({ ...prev, hidden_tools: event.target.value }))}
                    placeholder={wp.hiddenToolsPlaceholder}
                  />
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">{wp.forcedToolsLabel}</div>
                  <Input
                    value={form.forced_tools}
                    onChange={(event) => setForm((prev) => ({ ...prev, forced_tools: event.target.value }))}
                    placeholder={wp.forcedToolsPlaceholder}
                  />
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
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
  const tools = resolveWorkerVisibleTools(worker)
  return {
    id: worker.name,
    title: worker.display_name || worker.name,
    subtitle: worker.name,
    icon: meta.icon,
    colorClass: meta.colorClass,
    status: meta.status,
    statusLabel: meta.statusLabel,
    systemPrompt: worker.system_prompt,
    tools: tools.map((toolName) => ({
      name: toolName,
      icon: isBuiltinWorkerToolName(toolName) ? TOOL_ICON_BY_NAME[toolName] : Globe,
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
