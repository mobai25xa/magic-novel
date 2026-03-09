import { AlignCenter, AlignLeft, Baseline, BookOpen, Eye, Heart, IndentIncrease, Languages, MoveHorizontal, Package, Palette, Rows3, Sparkles, Sun, Moon, Monitor, Target, Type } from 'lucide-react'

import { Input, Toggle } from '@/magic-ui/components'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/magic-ui/components'
import { Slider } from '@/magic-ui/components'
import { EDITOR_FONT_PRESETS, type EditorFontPresetKey } from '@/state/settings'
import type { Language } from '@/types/theme'

import { renderAiSettings } from './settings-dialog-ai-content'
import { renderProjectsContent } from './settings-dialog-projects-content'
import { renderProvidersContent } from './settings-dialog-providers-content'
import type { SettingsDialogTranslations, SettingsTabId } from './settings-dialog-types'
import type { TempState } from './use-settings-dialog-controller'

const APP_INFO = {
  name: 'Magic Novel',
  version: '0.1.0',
  identifier: 'com.magic-novel.app',
}

export function renderSettingsDialogContent(input: {
  activeTab: SettingsTabId
  translations: SettingsDialogTranslations
  language: string
  temp: TempState
  onSelectDirectory: () => Promise<void>
  onFetchModels: () => Promise<void>
  onFetchEmbeddingModels: () => Promise<void>
  resetProjectGenres: () => void
}) {
  switch (input.activeTab) {
    case 'general':
      return renderGeneral(input.translations, input.temp)
    case 'providers':
      return renderProvidersContent(
        input.translations,
        input.temp,
        input.onFetchModels,
        input.onFetchEmbeddingModels,
      )
    case 'editor':
      return renderEditor(input.translations, input.language, input.temp)
    case 'projects':
      return renderProjectsContent(input.translations, input.temp, input.onSelectDirectory, input.resetProjectGenres)
    case 'ai':
      return renderAiSettings(input.translations, input.temp)
    default:
      return renderAbout(input.translations)
  }
}

/* ── P3-B2: About 子页面 ── */
function renderAbout(translations: SettingsDialogTranslations) {
  return (
    <>
      {/* App Hero */}
      <div className="settings-card" style={{ display: 'flex', alignItems: 'center', gap: 28, padding: '32px 36px', background: 'linear-gradient(135deg, var(--bg-white) 0%, rgba(5,150,105,0.03) 100%)' }}>
        <div style={{ width: 72, height: 72, borderRadius: 16, background: 'linear-gradient(135deg, var(--set-accent), var(--set-accent-dark))', display: 'flex', alignItems: 'center', justifyContent: 'center', boxShadow: '0 8px 24px rgba(5,150,105,0.3)', flexShrink: 0 }}>
          <BookOpen size={36} color="white" />
        </div>
        <div>
          <h2 style={{ fontSize: 26, fontWeight: 800, color: 'var(--text-main)', margin: '0 0 6px' }}>{APP_INFO.name}</h2>
          <p style={{ fontSize: 14, color: 'var(--text-secondary)', margin: 0, lineHeight: 1.5 }}>{translations.settings.appDescription}</p>
          <div style={{ marginTop: 12 }}>
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6, padding: '4px 12px', borderRadius: 6, background: 'var(--set-accent-light)', color: 'var(--set-accent)', fontSize: 12, fontWeight: 700 }}>
              <Sparkles size={12} /> v{APP_INFO.version} Early Access
            </span>
          </div>
        </div>
      </div>

      {/* App Info */}
      <div className="settings-card">
        <div className="card-header">
          <div className="card-icon"><Package size={18} /></div>
          <h2 className="card-title">{translations.settings.appInfo}</h2>
        </div>
        <div style={{ marginTop: 16 }}>
          <InfoRow label={translations.settings.appName} value={APP_INFO.name} />
          <InfoRow label={translations.settings.currentVersion} value={APP_INFO.version} />
          <InfoRow label={translations.settings.appIdentifier} value={APP_INFO.identifier} isLast />
        </div>
      </div>

      {/* Acknowledgements */}
      <div className="settings-card" style={{ background: 'linear-gradient(135deg, var(--bg-white) 0%, rgba(59,130,246,0.03) 100%)' }}>
        <div className="card-header">
          <div className="card-icon" style={{ background: 'rgba(59,130,246,0.1)', color: '#3b82f6' }}><Heart size={18} /></div>
          <h2 className="card-title">{translations.settingsExtra.acknowledgments}</h2>
        </div>
        <p style={{ fontSize: 13, color: 'var(--text-muted)', margin: '12px 0 0', lineHeight: 1.7 }}>
          {translations.settingsExtra.poweredBy}
        </p>
      </div>
    </>
  )
}

function InfoRow({ label, value, isLast }: { label: string; value: string; isLast?: boolean }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '14px 0', borderBottom: isLast ? 'none' : '1px solid rgba(0,0,0,0.04)' }}>
      <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)' }}>{label}</span>
      <span style={{ fontSize: 13, fontWeight: 700, color: 'var(--text-main)', fontFamily: "'Inter', monospace" }}>{value}</span>
    </div>
  )
}

/* ── P3-B3: General 子页面 ── */
function renderGeneral(translations: SettingsDialogTranslations, temp: TempState) {
  const t = translations.settings
  return (
    <>
      {/* Theme */}
      <div className="bento-card">
        <div className="card-header">
          <div className="card-icon"><Palette size={18} /></div>
          <h2 className="card-title">{t.theme}</h2>
        </div>
        <p className="card-desc">{t.themeDescription}</p>
        <div className="segmented-control">
          <button className={`seg-btn ${temp.tempTheme === 'light' ? 'active' : ''}`} onClick={() => temp.setTempTheme('light')}>
            <Sun size={15} /> {t.themeLight}
          </button>
          <button className={`seg-btn ${temp.tempTheme === 'dark' ? 'active' : ''}`} onClick={() => temp.setTempTheme('dark')}>
            <Moon size={15} /> {t.themeDark}
          </button>
          <button className={`seg-btn ${temp.tempTheme === 'system' ? 'active' : ''}`} onClick={() => temp.setTempTheme('system')}>
            <Monitor size={15} /> {t.themeSystem}
          </button>
        </div>
      </div>

      {/* Language */}
      <div className="bento-card">
        <div className="card-header">
          <div className="card-icon" style={{ background: 'rgba(59,130,246,0.1)', color: '#3b82f6' }}><Languages size={18} /></div>
          <h2 className="card-title">{t.language}</h2>
        </div>
        <p className="card-desc">{t.languageDescription}</p>
        <Select value={temp.tempLanguage} onValueChange={(v) => temp.setTempLanguage(v as Language)}>
          <SelectTrigger style={{ width: 200 }}><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="zh">{translations.settingsExtra.zhCN}</SelectItem>
            <SelectItem value="en">English</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Writing Goal */}
      <div className="bento-card">
        <div className="card-header">
          <div className="card-icon" style={{ background: 'rgba(16,185,129,0.1)', color: '#10b981' }}><Target size={18} /></div>
          <h2 className="card-title">{t.writingGoal}</h2>
        </div>
        <p className="card-desc">{t.writingGoalDescription}</p>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Input type="number" value={temp.tempGoal} onChange={(e) => temp.setTempGoal(Number(e.target.value))} min={1} style={{ width: 140 }} />
          <span style={{ fontSize: 13, color: 'var(--text-muted)', fontWeight: 600 }}>{t.wordsPerDay}</span>
        </div>
      </div>
    </>
  )
}

/* ── P3-B4~B7: Editor 子页面 ── */
function renderEditor(translations: SettingsDialogTranslations, language: string, temp: TempState) {
  const t = translations.settings
  const lang = language as 'zh' | 'en'
  const fontPresetEntries = Object.entries(EDITOR_FONT_PRESETS) as [EditorFontPresetKey, (typeof EDITOR_FONT_PRESETS)[EditorFontPresetKey]][]

  return (
    <>
      {/* Text Align & Font — 2-column */}
      <div className="bento-grid-2">
        <div className="bento-card">
          <div className="card-header">
            <div className="card-icon"><AlignCenter size={18} /></div>
            <h2 className="card-title">{t.textAlign}</h2>
          </div>
          <p className="card-desc" style={{ marginLeft: 0, marginTop: 8 }}>{t.textAlignDescription}</p>
          <div className="segmented-control">
            <button className={`seg-btn ${temp.tempEditorTextAlign === 'center' ? 'active' : ''}`} onClick={() => temp.setTempEditorTextAlign('center')}>
              <AlignCenter size={14} /> {t.textAlignCenter}
            </button>
            <button className={`seg-btn ${temp.tempEditorTextAlign === 'left' ? 'active' : ''}`} onClick={() => temp.setTempEditorTextAlign('left')}>
              <AlignLeft size={14} /> {t.textAlignLeft}
            </button>
          </div>
        </div>

        <div className="bento-card">
          <div className="card-header">
            <div className="card-icon" style={{ background: 'rgba(59,130,246,0.1)', color: '#3b82f6' }}><Type size={18} /></div>
            <h2 className="card-title">{t.fontFamily}</h2>
          </div>
          <p className="card-desc" style={{ marginLeft: 0, marginTop: 8 }}>{t.fontFamilyDescription}</p>
          <Select value={temp.tempEditorFontFamily} onValueChange={(v) => temp.setTempEditorFontFamily(v as EditorFontPresetKey)}>
            <SelectTrigger style={{ width: '100%' }}><SelectValue /></SelectTrigger>
            <SelectContent>
              {fontPresetEntries.map(([key, preset]) => (
                <SelectItem key={key} value={key}>{preset.label[lang] || preset.label.zh}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Font Size */}
      <div className="bento-card" style={{ marginTop: 16 }}>
        <div className="card-header">
          <div className="card-icon" style={{ background: 'rgba(16,185,129,0.1)', color: '#10b981' }}><Baseline size={18} /></div>
          <h2 className="card-title">{t.fontSize}</h2>
        </div>
        <p className="card-desc">{t.fontSizeDescription}</p>
        <div className="slider-row">
          <span style={{ fontSize: 12, color: 'var(--text-muted)', fontWeight: 600, width: 28 }}>12px</span>
          <Slider value={[temp.tempEditorFontSize]} min={12} max={24} step={1} onValueChange={([v]) => temp.setTempEditorFontSize(v)} />
          <span style={{ fontSize: 12, color: 'var(--text-muted)', fontWeight: 600, width: 28, textAlign: 'right' }}>24px</span>
          <span className="slider-value">{temp.tempEditorFontSize}px</span>
        </div>
      </div>

      {/* Line Height & Content Width — 2-column */}
      <div className="bento-grid-2" style={{ marginTop: 16 }}>
        <div className="bento-card">
          <div className="card-header">
            <div className="card-icon"><Rows3 size={18} /></div>
            <h2 className="card-title">{t.lineHeight}</h2>
          </div>
          <p className="card-desc" style={{ marginLeft: 0, marginTop: 8 }}>{t.lineHeightDescription}</p>
          <div className="slider-row">
            <Slider value={[temp.tempEditorLineHeight]} min={1.2} max={2.5} step={0.1} onValueChange={([v]) => temp.setTempEditorLineHeight(v)} />
            <span className="slider-value">{temp.tempEditorLineHeight.toFixed(1)}</span>
          </div>
        </div>

        <div className="bento-card">
          <div className="card-header">
            <div className="card-icon" style={{ background: 'rgba(168,85,247,0.1)', color: '#a855f7' }}><MoveHorizontal size={18} /></div>
            <h2 className="card-title">{t.contentWidth}</h2>
          </div>
          <p className="card-desc" style={{ marginLeft: 0, marginTop: 8 }}>{t.contentWidthDescription}</p>
          <div className="slider-row">
            <Slider value={[temp.tempEditorContentWidth]} min={500} max={1000} step={50} onValueChange={([v]) => temp.setTempEditorContentWidth(v)} />
            <span className="slider-value">{temp.tempEditorContentWidth}px</span>
          </div>
        </div>
      </div>

      {/* First Line Indent */}
      <div className="bento-card" style={{ marginTop: 16 }}>
        <div className="toggle-row">
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div className="card-icon" style={{ background: 'rgba(245,158,11,0.1)', color: '#f59e0b' }}><IndentIncrease size={18} /></div>
            <div>
              <div className="toggle-label">{t.firstLineIndent}</div>
              <div className="toggle-desc">{t.firstLineIndentDescription}</div>
            </div>
          </div>
          <Toggle checked={temp.tempFirstLineIndent} onChange={(e) => temp.setTempFirstLineIndent(e.target.checked)} />
        </div>
      </div>

      {/* Live Preview */}
      <div className="bento-card" style={{ marginTop: 16, background: 'linear-gradient(135deg, var(--bg-white) 0%, rgba(5,150,105,0.02) 100%)' }}>
        <div className="card-header">
          <div className="card-icon"><Eye size={18} /></div>
          <h2 className="card-title">{t.livePreview}</h2>
        </div>
        <div style={{ border: '1px solid var(--border-color)', borderRadius: 8, padding: '20px 24px', marginTop: 16, background: 'var(--bg-app)', transition: 'all 0.3s' }}>
          <p style={{
            fontSize: temp.tempEditorFontSize,
            lineHeight: temp.tempEditorLineHeight,
            textIndent: temp.tempFirstLineIndent ? '2em' : '0',
            maxWidth: temp.tempEditorContentWidth,
            margin: temp.tempEditorTextAlign === 'center' ? '0 auto' : '0',
            fontFamily: EDITOR_FONT_PRESETS[temp.tempEditorFontFamily]?.fontFamily,
            color: 'var(--text-main)',
          }}>
            {t.previewText}
          </p>
        </div>
      </div>
    </>
  )
}
