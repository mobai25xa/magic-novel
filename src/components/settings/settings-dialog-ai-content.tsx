/* eslint-disable react-hooks/set-state-in-effect */
import { useCallback, useEffect, useState } from 'react'
import { Trash2, Bot, FileText, Shield, CheckCircle2, PencilLine } from 'lucide-react'

import {
  deleteSkillFeature,
  deleteWorkerFeature,
  getGlobalRulesFeature,
  listSkillsFeature,
  listWorkersFeature,
  resolveWorkerVisibleTools,
  saveGlobalRulesFeature,
  type SkillDefinition,
  type WorkerDefinition,
} from '@/features/global-config'
import { Badge, Button, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Textarea } from '@/magic-ui/components'
import type { SettingsDialogTranslations } from './settings-dialog-types'
import type { TempState } from './use-settings-dialog-controller'

type LoadState = 'idle' | 'loading' | 'loaded' | 'error'

export function renderAiSettings(translations: SettingsDialogTranslations, temp: TempState) {
  return <AiSettingsPanel key="ai-settings-panel" translations={translations} temp={temp} />
}

function AiSettingsPanel({ translations, temp }: { translations: SettingsDialogTranslations; temp: TempState }) {
  const t = translations.settings

  const [loadState, setLoadState] = useState<LoadState>('loading')
  const [skills, setSkills] = useState<SkillDefinition[]>([])
  const [workers, setWorkers] = useState<WorkerDefinition[]>([])
  const [globalRules, setGlobalRules] = useState('')
  const [rulesSaved, setRulesSaved] = useState(false)

  const loadConfig = useCallback(async () => {
    try {
      const [sk, wk, rules] = await Promise.all([
        listSkillsFeature(),
        listWorkersFeature(),
        getGlobalRulesFeature(),
      ])
      setSkills(sk)
      setWorkers(wk)
      setGlobalRules(rules || '')
      setLoadState('loaded')
    } catch {
      setLoadState('error')
    }
  }, [])

  useEffect(() => {
    void loadConfig()
  }, [loadConfig])

  const handleDeleteSkill = async (name: string) => {
    if (!confirm(t.aiSkillDeleteConfirm)) return
    await deleteSkillFeature(name)
    setSkills((prev) => prev.filter((s) => s.name !== name))
  }

  const handleDeleteWorker = async (name: string) => {
    if (!confirm(t.aiWorkerDeleteConfirm)) return
    await deleteWorkerFeature(name)
    setWorkers((prev) => prev.filter((w) => w.name !== name))
  }

  const handleSaveRules = async () => {
    await saveGlobalRulesFeature(globalRules)
    setRulesSaved(true)
    setTimeout(() => setRulesSaved(false), 2000)
  }

  if (loadState === 'loading') {
    return <div className="text-sm text-muted-foreground py-8 text-center">{t.aiConfigLoading}</div>
  }
  if (loadState === 'error') {
    return <div className="text-sm text-destructive py-8 text-center">{t.aiConfigError}</div>
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-2">{t.aiSettings || 'AI Chat'}</h3>
        <p className="text-sm text-muted-foreground">{t.aiSettingsDescription}</p>
      </div>
      {renderPolicySection(t, temp)}
      {renderSkillsSection(t, skills, handleDeleteSkill)}
      {renderWorkersSection(t, workers, handleDeleteWorker)}
      {renderGlobalRulesSection(t, globalRules, setGlobalRules, handleSaveRules, rulesSaved)}
    </div>
  )
}

function renderPolicySection(t: Record<string, string>, temp: TempState) {
  return (
    <div className="bento-card space-y-5">
      <div>
        <h4 className="text-sm font-semibold">{t.aiRunPolicyTitle}</h4>
        <p className="text-xs text-muted-foreground mt-1">{t.aiRunPolicyDescription}</p>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm font-medium">
            <CheckCircle2 className="h-4 w-4 text-muted-foreground" />
            <span>{t.aiApprovalModeLabel}</span>
          </div>
          <p className="text-xs text-muted-foreground">{t.aiApprovalModeDescription}</p>
          <Select value={temp.tempApprovalMode} onValueChange={(value) => temp.setTempApprovalMode(value as typeof temp.tempApprovalMode)}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder={t.aiApprovalModePlaceholder} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="confirm_writes">{t.aiApprovalModeConfirmWrites}</SelectItem>
              <SelectItem value="auto">{t.aiApprovalModeAutoRun}</SelectItem>
            </SelectContent>
          </Select>
          <Badge color={temp.tempApprovalMode === 'auto' ? 'warning' : 'info'} variant="soft">
            {temp.tempApprovalMode === 'auto' ? t.aiApprovalModeAutoRun : t.aiApprovalModeConfirmWrites}
          </Badge>
        </div>

        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm font-medium">
            <PencilLine className="h-4 w-4 text-muted-foreground" />
            <span>{t.aiCapabilityModeLabel}</span>
          </div>
          <p className="text-xs text-muted-foreground">{t.aiCapabilityModeDescription}</p>
          <Select value={temp.tempCapabilityMode} onValueChange={(value) => temp.setTempCapabilityMode(value as typeof temp.tempCapabilityMode)}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder={t.aiCapabilityModePlaceholder} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="writing">{t.aiCapabilityModeWriting}</SelectItem>
              <SelectItem value="planning">{t.aiCapabilityModePlanning}</SelectItem>
            </SelectContent>
          </Select>
          <Badge color={temp.tempCapabilityMode === 'planning' ? 'warning' : 'success'} variant="soft">
            {temp.tempCapabilityMode === 'planning' ? t.aiCapabilityModePlanning : t.aiCapabilityModeWriting}
          </Badge>
        </div>
      </div>
    </div>
  )
}

function renderSkillsSection(
  t: Record<string, string>,
  skills: SkillDefinition[],
  onDelete: (name: string) => void,
) {
  return (
    <div className="settings-section">
      <div className="flex items-center gap-2 mb-3">
        <FileText className="h-4 w-4 text-muted-foreground" />
        <div>
          <h4 className="text-sm font-medium">{t.aiSkillsTitle}</h4>
          <p className="text-xs text-muted-foreground">{t.aiSkillsDescription}</p>
        </div>
      </div>
      {skills.length === 0 ? (
        <p className="text-xs text-muted-foreground italic">{t.aiSkillEmpty}</p>
      ) : (
        <div className="space-y-2">
          {skills.map((skill) => (
            <div key={skill.name} className="flex items-center justify-between settings-section px-3 py-2">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium truncate">{skill.display_name}</span>
                  <span className={`text-[10px] px-1.5 py-0.5 rounded ${skill.source === 'builtin' ? 'model-tag-info' : 'model-tag-success'}`}>
                    {skill.source === 'builtin' ? t.aiSkillBuiltin : t.aiSkillUser}
                  </span>
                </div>
                {skill.description && (
                  <p className="text-xs text-muted-foreground truncate mt-0.5">{skill.description}</p>
                )}
              </div>
              {skill.source === 'user' && (
                <button onClick={() => onDelete(skill.name)} className="ml-2 p-1 text-muted-foreground hover:text-destructive transition-colors">
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
function renderWorkersSection(
  t: Record<string, string>,
  workers: WorkerDefinition[],
  onDelete: (name: string) => void,
) {
  return (
    <div className="settings-section">
      <div className="flex items-center gap-2 mb-3">
        <Bot className="h-4 w-4 text-muted-foreground" />
        <div>
          <h4 className="text-sm font-medium">{t.aiWorkersTitle}</h4>
          <p className="text-xs text-muted-foreground">{t.aiWorkersDescription}</p>
        </div>
      </div>
      {workers.length === 0 ? (
        <p className="text-xs text-muted-foreground italic">{t.aiWorkerEmpty}</p>
      ) : (
        <div className="space-y-2">
          {workers.map((worker) => (
            <div key={worker.name} className="flex items-center justify-between settings-section px-3 py-2">
              <div className="flex-1 min-w-0">
                <span className="text-sm font-medium">{worker.display_name}</span>
                <div className="flex flex-wrap gap-1 mt-1">
                  {resolveWorkerVisibleTools(worker).slice(0, 5).map((tool) => (
                    <span key={tool} className="tag tag-hover">{tool}</span>
                  ))}
                  {resolveWorkerVisibleTools(worker).length > 5 && (
                    <span className="tag tag-hover">+{resolveWorkerVisibleTools(worker).length - 5}</span>
                  )}
                </div>
                <div className="flex flex-wrap gap-1 mt-1">
                  <span className="tag tag-warning">{worker.capability_preset}</span>
                  {worker.allow_delegate && <span className="tag tag-warning">delegate</span>}
                  {worker.allow_skill_activation && <span className="tag tag-warning">skill</span>}
                </div>
              </div>
              <button onClick={() => onDelete(worker.name)} className="ml-2 p-1 text-muted-foreground hover:text-destructive transition-colors">
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function renderGlobalRulesSection(
  t: Record<string, string>,
  globalRules: string,
  setGlobalRules: (v: string) => void,
  onSave: () => void,
  saved: boolean,
) {
  return (
    <div className="settings-section">
      <div className="flex items-center gap-2 mb-3">
        <Shield className="h-4 w-4 text-muted-foreground" />
        <div>
          <h4 className="text-sm font-medium">{t.aiGlobalRulesTitle}</h4>
          <p className="text-xs text-muted-foreground">{t.aiGlobalRulesDescription}</p>
        </div>
      </div>
      <Textarea
        value={globalRules}
        onChange={(e) => setGlobalRules(e.target.value)}
        placeholder={t.aiGlobalRulesPlaceholder}
        className="min-h-[120px]"
      />
      <div className="flex justify-end mt-2">
        <Button
          onClick={onSave}
          size="sm"
        >
          {saved ? t.aiGlobalRulesSaved : t.aiGlobalRulesSave}
        </Button>
      </div>
    </div>
  )
}
