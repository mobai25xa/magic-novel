import { useCallback, useEffect, useMemo, useState } from 'react'
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { Plus, AlertCircle, RefreshCw } from 'lucide-react'

import { useTranslation } from '@/hooks/use-translation'
import {
  Skeleton,
  toast,
  Modal,
  ModalContent,
  ModalHeader,
  ModalTitle,
  ModalDescription,
  ModalFooter,
  Button,
  Input,
  Textarea,
} from '@/magic-ui/components'
import {
  listSkillsFeature,
  saveSkillFeature,
  deleteSkillFeature,
  importSkillFeature,
  exportSkillFeature,
  type SkillDefinition,
} from '@/features/global-config'

import { SkillHero } from './SkillHero'
import { SkillFilterBar } from './SkillFilterBar'
import { SkillCard } from './SkillCard'
import { SkillEmptyState } from './SkillEmptyState'
import type { SkillCategory, SkillColorVariant } from './types'

function detectSkillCategory(skill: SkillDefinition): SkillCategory {
  const text = `${skill.name} ${skill.display_name} ${skill.description}`.toLowerCase()

  if (/润色|改写|polish|rewrite|proof|edit/.test(text)) {
    return 'polish'
  }

  if (/逻辑|推演|reason|logic|plot|审查|漏洞/.test(text)) {
    return 'logic'
  }

  if (/角色|扮演|人设|dialog|role|persona/.test(text)) {
    return 'roleplay'
  }

  return 'all'
}

function normalizeSkillName(raw: string): string {
  return raw
    .trim()
    .toLowerCase()
    .replace(/\s+/g, '-')
    .replace(/[^a-z0-9-_]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
}

function buildTemplateMarkdown(name: string): string {
  const display = name || 'New Skill'
  return `# ${display}\n\nDescribe what this skill does and when it should be used.\n\n## Instructions\n- Keep responses concise\n- Focus on writing quality\n`
}

export function SkillsPage() {
  const { translations } = useTranslation()
  const tw = translations.skillsWorkshop

  const [skills, setSkills] = useState<SkillDefinition[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(false)
  const [activeFilter, setActiveFilter] = useState<SkillCategory>('all')

  const [editorOpen, setEditorOpen] = useState(false)
  const [editingSkillName, setEditingSkillName] = useState<string | null>(null)
  const [formName, setFormName] = useState('')
  const [formContent, setFormContent] = useState('')
  const [saving, setSaving] = useState(false)

  const [deletingSkill, setDeletingSkill] = useState<SkillDefinition | null>(null)

  const loadSkills = useCallback(async () => {
    setLoading(true)
    setError(false)
    try {
      const list = await listSkillsFeature()
      setSkills(list)
    } catch {
      setError(true)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void loadSkills()
  }, [loadSkills])

  const handleToggle = useCallback((name: string, enabled: boolean) => {
    setSkills((prev) => prev.map((s) => (s.name === name ? { ...s, enabled } : s)))
  }, [])

  const openCreateDialog = useCallback(() => {
    setEditingSkillName(null)
    setFormName('')
    setFormContent(buildTemplateMarkdown(''))
    setEditorOpen(true)
  }, [])

  const openEditDialog = useCallback((skill: SkillDefinition) => {
    setEditingSkillName(skill.name)
    setFormName(skill.name)
    setFormContent(skill.system_prompt_snippet || buildTemplateMarkdown(skill.display_name || skill.name))
    setEditorOpen(true)
  }, [])

  const handleSaveSkill = useCallback(async () => {
    const normalizedName = normalizeSkillName(formName)
    if (!normalizedName) {
      toast.warning(tw.nameRequired)
      return
    }

    if (!formContent.trim()) {
      toast.warning(tw.contentRequired)
      return
    }

    setSaving(true)
    try {
      await saveSkillFeature(normalizedName, formContent)

      if (editingSkillName && editingSkillName !== normalizedName) {
        await deleteSkillFeature(editingSkillName)
      }

      setEditorOpen(false)
      await loadSkills()
      toast.success(tw.saved)
    } catch (e) {
      toast.error(tw.saveFailed, String(e))
    } finally {
      setSaving(false)
    }
  }, [editingSkillName, formName, formContent, loadSkills, tw.contentRequired, tw.nameRequired, tw.saveFailed, tw.saved])

  const handleDelete = useCallback(async () => {
    if (!deletingSkill) return

    try {
      await deleteSkillFeature(deletingSkill.name)
      setDeletingSkill(null)
      await loadSkills()
      toast.success(tw.deleted)
    } catch (e) {
      toast.error(tw.operationFailed, String(e))
    }
  }, [deletingSkill, loadSkills, tw.deleted, tw.operationFailed])

  const handleImportSkill = useCallback(async () => {
    try {
      const selected = await openDialog({
        title: tw.importDialogTitle,
        multiple: false,
        directory: false,
        filters: [{ name: 'Markdown', extensions: ['md'] }],
      })

      if (!selected || typeof selected !== 'string') return

      await importSkillFeature(selected)
      await loadSkills()
      toast.success(tw.importSuccess)
    } catch (e) {
      toast.error(tw.operationFailed, String(e))
    }
  }, [loadSkills, tw.importDialogTitle, tw.importSuccess, tw.operationFailed])

  const handleExportSkill = useCallback(async (skill: SkillDefinition) => {
    try {
      const outputPath = await saveDialog({
        title: tw.exportDialogTitle,
        filters: [{ name: 'Markdown', extensions: ['md'] }],
        defaultPath: `${skill.name}.md`,
      })

      if (!outputPath || typeof outputPath !== 'string') return

      await exportSkillFeature(skill.name, outputPath)
      toast.success(tw.exportSuccess)
    } catch (e) {
      toast.error(tw.operationFailed, String(e))
    }
  }, [tw.exportDialogTitle, tw.exportSuccess, tw.operationFailed])

  const handleOpenMarket = useCallback(() => {
    toast.info(tw.filterMarket)
  }, [tw.filterMarket])

  const filteredSkills = useMemo(() => {
    if (activeFilter === 'all') {
      return skills
    }

    return skills.filter((skill) => detectSkillCategory(skill) === activeFilter)
  }, [activeFilter, skills])

  return (
    <div className="skills-page">
      <div className="bento-grid bento-grid-skills">
        <SkillHero tw={tw} />
        <SkillFilterBar
          tw={tw}
          active={activeFilter}
          onChange={setActiveFilter}
          onOpenImport={handleImportSkill}
          onOpenMarket={handleOpenMarket}
        />

        {loading && (
          <>
            {[1, 2, 3].map((i) => (
              <div key={i} className="bento-card skill-skeleton-card span-4">
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <Skeleton width={48} height={48} style={{ borderRadius: 16 }} />
                  <Skeleton width={40} height={22} style={{ borderRadius: 11 }} />
                </div>
                <Skeleton variant="heading" width="70%" />
                <Skeleton variant="text" lines={2} />
                <div style={{ marginTop: 'auto', paddingTop: 16, borderTop: '1px dashed var(--border-color)' }}>
                  <Skeleton width={60} height={22} style={{ borderRadius: 6 }} />
                </div>
              </div>
            ))}
          </>
        )}

        {!loading && error && (
          <div className="bento-error-state span-12">
            <div className="bento-error-icon">
              <AlertCircle size={32} />
            </div>
            <p>{tw.loadFailed}</p>
            <button className="btn btn-solid-success" onClick={loadSkills}>
              <RefreshCw size={16} />
              {tw.retry}
            </button>
          </div>
        )}

        {!loading && !error && (
          <>
            {filteredSkills.map((skill, i) => (
              <SkillCard
                key={skill.name}
                skill={skill}
                colorVariant={((i % 5) + 1) as SkillColorVariant}
                tw={tw}
                onToggle={handleToggle}
                onEdit={openEditDialog}
                onExport={handleExportSkill}
                onDelete={(s) => setDeletingSkill(s)}
              />
            ))}

            <div
              className="bento-card skill-card skill-card-create span-4"
              role="button"
              tabIndex={0}
              onClick={openCreateDialog}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  openCreateDialog()
                }
              }}
            >
              <div className="skill-create-icon">
                <Plus size={28} />
              </div>
              <h3>{tw.createCardTitle}</h3>
              <p>{tw.createCardDescription}</p>
            </div>

            {filteredSkills.length === 0 && (
              <SkillEmptyState
                tw={tw}
                isFilterResult={activeFilter !== 'all'}
                onCreate={openCreateDialog}
              />
            )}
          </>
        )}
      </div>

      <Modal
        open={editorOpen}
        onOpenChange={(open) => {
          if (!open && saving) return
          setEditorOpen(open)
        }}
      >
        <ModalContent size="lg">
          <ModalHeader>
            <ModalTitle>{editingSkillName ? tw.editSkill : tw.createSkill}</ModalTitle>
            <ModalDescription>{tw.editorHint}</ModalDescription>
          </ModalHeader>
          <div className="p-6 space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">{tw.nameLabel}</label>
              <Input
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder={tw.namePlaceholder}
                disabled={saving}
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-2">{tw.contentLabel}</label>
              <Textarea
                value={formContent}
                onChange={(e) => setFormContent(e.target.value)}
                placeholder={tw.contentPlaceholder}
                className="min-h-[260px]"
                disabled={saving}
              />
            </div>
          </div>
          <ModalFooter>
            <Button variant="secondary" onClick={() => setEditorOpen(false)} disabled={saving}>
              {translations.common.cancel}
            </Button>
            <Button onClick={() => void handleSaveSkill()} disabled={saving}>
              {saving ? translations.common.loading : translations.common.save}
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>

      <Modal open={!!deletingSkill} onOpenChange={(open) => !open && setDeletingSkill(null)}>
        <ModalContent size="sm">
          <ModalHeader>
            <ModalTitle>{tw.deleteSkill}</ModalTitle>
            <ModalDescription>
              {deletingSkill?.source === 'builtin' ? tw.deleteBuiltinConfirm : tw.deleteConfirm}
            </ModalDescription>
          </ModalHeader>
          <ModalFooter>
            <Button variant="secondary" onClick={() => setDeletingSkill(null)}>
              {translations.common.cancel}
            </Button>
            <Button variant="destructive" onClick={() => void handleDelete()}>
              {translations.common.delete}
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  )
}
