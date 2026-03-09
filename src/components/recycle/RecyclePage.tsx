import { useState, useCallback, useEffect } from 'react'
import { BookOpen, FileText, Layers, Package, Clock3, Info, AlertCircle, RefreshCw } from 'lucide-react'

import { useTranslation } from '@/hooks/use-translation'
import { eventBus, EVENTS } from '@/lib/events'
import {
  emptyRecycleBinByProject,
  emptyRecycledProjectsByRoot,
  listRecycleItemsByProject,
  listRecycledProjectsByRoot,
  permanentlyDeleteRecycleItemById,
  permanentlyDeleteRecycledProjectById,
  restoreRecycleItemById,
  restoreRecycledProjectById,
} from '@/features/recycle'
import { useProjectStore } from '@/state/project'
import { useSettingsStore } from '@/state/settings'
import { Skeleton } from '@/magic-ui/components'
import { toast } from '@/magic-ui/components'
import { ConfirmDialog } from '@/components/common/ConfirmDialog'
import { RecycleHero } from './RecycleHero'
import { RecycleCard } from './RecycleCard'
import { RecycleEmptyState } from './RecycleEmptyState'
import type { RecycleItem, RecycleItemType } from './types'
import { getSeverity } from './types'

type FilterType = RecycleItemType | 'all'

const FILTER_KEYS: { key: FilterType; labelKey: string; icon?: typeof BookOpen }[] = [
  { key: 'all', labelKey: 'filterAll', icon: Layers },
  { key: 'novel', labelKey: 'filterNovel', icon: BookOpen },
  { key: 'chapter', labelKey: 'filterChapter', icon: FileText },
  { key: 'volume', labelKey: 'filterVolume', icon: Layers },
]

type ConfirmAction =
  | { type: 'delete'; id: string; source: 'project' | 'workspace' }
  | { type: 'emptyAll' }
  | null

export function RecyclePage() {
  const { translations } = useTranslation()
  const tw = translations.recyclePage
  const projectPath = useProjectStore((s) => s.projectPath)
  const projectsRootDir = useSettingsStore((s) => s.projectsRootDir)

  const [items, setItems] = useState<RecycleItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(false)
  const [activeFilter, setActiveFilter] = useState<FilterType>('all')
  const [searchQuery, setSearchQuery] = useState('')
  const [confirmAction, setConfirmAction] = useState<ConfirmAction>(null)

  const typeFilteredItems = activeFilter === 'all'
    ? items
    : items.filter((i) => i.type === activeFilter)

  const normalizedQuery = searchQuery.trim().toLowerCase()
  const filteredItems = normalizedQuery.length === 0
    ? typeFilteredItems
    : typeFilteredItems.filter((item) =>
      [item.name, item.origin, item.description]
        .join(' ')
        .toLowerCase()
        .includes(normalizedQuery)
    )

  const expiringCount = items.filter((i) => getSeverity(i.daysRemaining) !== 'safe').length

  const handleRestore = useCallback(async (item: RecycleItem) => {
    try {
      if (item.source === 'workspace') {
        if (!projectsRootDir) return
        await restoreRecycledProjectById(projectsRootDir, item.id)
      } else {
        if (!projectPath) return
        await restoreRecycleItemById(projectPath, item.id)
      }
      setItems((prev) => prev.filter((i) => i.id !== item.id))
      toast.success(tw.restoreSuccess)
      eventBus.emit(EVENTS.RECYCLE_REFRESH_REQUESTED)
    } catch (error) {
      toast.error(String(error))
    }
  }, [projectPath, projectsRootDir, tw.restoreSuccess])

  const handleDeleteRequest = useCallback((item: RecycleItem) => {
    setConfirmAction({ type: 'delete', id: item.id, source: item.source })
  }, [])

  const handleConfirm = useCallback(async () => {
    if (!confirmAction) return

    try {
      if (confirmAction.type === 'delete') {
        if (confirmAction.source === 'workspace') {
          if (!projectsRootDir) return
          await permanentlyDeleteRecycledProjectById(projectsRootDir, confirmAction.id)
        } else {
          if (!projectPath) return
          await permanentlyDeleteRecycleItemById(projectPath, confirmAction.id)
        }
        setItems((prev) => prev.filter((i) => i.id !== confirmAction.id))
        toast.success(tw.deleteSuccess)
      } else {
        const hasWorkspaceItems = items.some((item) => item.source === 'workspace')
        const hasProjectItems = items.some((item) => item.source === 'project')

        if (hasWorkspaceItems) {
          if (!projectsRootDir) return
          await emptyRecycledProjectsByRoot(projectsRootDir)
        }
        if (hasProjectItems) {
          if (!projectPath) return
          await emptyRecycleBinByProject(projectPath)
        }

        setItems([])
        toast.success(tw.emptyAllSuccess)
      }
      eventBus.emit(EVENTS.RECYCLE_REFRESH_REQUESTED)
    } catch (error) {
      toast.error(String(error))
    } finally {
      setConfirmAction(null)
    }
  }, [confirmAction, items, projectPath, projectsRootDir, tw.deleteSuccess, tw.emptyAllSuccess])

  const handleCancelConfirm = useCallback(() => {
    setConfirmAction(null)
  }, [])

  const loadItems = useCallback(async () => {
    const tasks: Array<Promise<{ source: 'project' | 'workspace'; items: Awaited<ReturnType<typeof listRecycleItemsByProject>> | Awaited<ReturnType<typeof listRecycledProjectsByRoot>> }>> = []

    if (projectPath) {
      tasks.push(listRecycleItemsByProject(projectPath).then((items) => ({ source: 'project' as const, items })))
    }

    if (projectsRootDir) {
      tasks.push(listRecycledProjectsByRoot(projectsRootDir).then((items) => ({ source: 'workspace' as const, items })))
    }

    if (tasks.length <= 0) {
      setItems([])
      return
    }

    setLoading(true)
    setError(false)
    try {
      const results = await Promise.all(tasks)
      const merged = results
        .flatMap((result) => result.items.map((item) => ({
          id: item.id,
          name: item.name,
          type: item.type,
          origin: item.origin,
          description: item.description,
          deletedAt: new Date(item.deleted_at).toISOString().slice(0, 10),
          deletedAtMs: item.deleted_at,
          daysRemaining: item.days_remaining,
          source: result.source,
        })))
        .sort((a, b) => b.deletedAtMs - a.deletedAtMs)
        .map(({ deletedAtMs: _deletedAtMs, ...item }) => item)

      setItems(merged)
    } catch {
      setError(true)
    } finally {
      setLoading(false)
    }
  }, [projectPath, projectsRootDir])

  useEffect(() => {
    void loadItems()
  }, [loadItems])

  useEffect(() => {
    const handleSearchChanged = (value?: unknown) => {
      setSearchQuery(typeof value === 'string' ? value : '')
    }

    const handleEmptyAllRequested = () => {
      if (items.length > 0) {
        setConfirmAction({ type: 'emptyAll' })
      }
    }

    const handleRefreshRequested = () => {
      void loadItems()
    }

    eventBus.on(EVENTS.RECYCLE_SEARCH_CHANGED, handleSearchChanged)
    eventBus.on(EVENTS.RECYCLE_EMPTY_ALL_REQUESTED, handleEmptyAllRequested)
    eventBus.on(EVENTS.RECYCLE_REFRESH_REQUESTED, handleRefreshRequested)

    return () => {
      eventBus.off(EVENTS.RECYCLE_SEARCH_CHANGED, handleSearchChanged)
      eventBus.off(EVENTS.RECYCLE_EMPTY_ALL_REQUESTED, handleEmptyAllRequested)
      eventBus.off(EVENTS.RECYCLE_REFRESH_REQUESTED, handleRefreshRequested)
    }
  }, [items.length, loadItems])

  return (
    <div className="recycle-page">
      <div className="ambient-glow" />
      <div className="ambient-glow-2" />

      <div className="content-scroll">
        <div className="bento-grid bento-grid-recycle">
          <RecycleHero tw={tw} />

          {/* 统计栏 */}
          {!loading && !error && items.length > 0 && (
            <div className="recycle-stats-bar span-12">
              <div className="recycle-stat-chip">
                <Package size={16} style={{ color: 'var(--color-danger)' }} />
                <span className="stat-num">{items.length}</span>
                {tw.statTotal}
              </div>
              <div className="recycle-stat-chip">
                <Clock3 size={16} style={{ color: 'var(--color-warning)' }} />
                <span className="stat-num">{expiringCount}</span>
                {tw.statExpiring}
              </div>
              <div className="recycle-stat-chip recycle-stat-info-chip">
                <Info size={14} />
                {tw.statAutoDeleteHint}
              </div>
            </div>
          )}

          {/* 过滤栏 */}
          {!loading && !error && items.length > 0 && (
            <div className="recycle-filter-bar span-12">
              {FILTER_KEYS.map((f) => (
                <button
                  key={f.key}
                  className={`recycle-filter-btn${activeFilter === f.key ? ' active' : ''}`}
                  onClick={() => setActiveFilter(f.key)}
                >
                  {f.icon && <f.icon size={16} />}
                  {tw[f.labelKey]}
                </button>
              ))}
            </div>
          )}

          {/* Loading skeleton */}
          {loading && (
            <>
              <div className="recycle-stats-bar span-12">
                <Skeleton width={140} height={36} style={{ borderRadius: 6 }} />
                <Skeleton width={140} height={36} style={{ borderRadius: 6 }} />
              </div>
              {[1, 2, 3].map((i) => (
                <div key={i} className="bento-card recycle-skeleton-card span-4">
                  <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                    <Skeleton width={48} height={48} style={{ borderRadius: 16 }} />
                    <Skeleton width={80} height={24} style={{ borderRadius: 20 }} />
                  </div>
                  <Skeleton variant="heading" width="60%" />
                  <Skeleton variant="text" width="40%" />
                  <Skeleton variant="text" lines={2} />
                  <div style={{ marginTop: 'auto', paddingTop: 16, borderTop: '1px dashed var(--border-color)', display: 'flex', justifyContent: 'space-between' }}>
                    <Skeleton width={80} height={14} />
                    <Skeleton width={100} height={28} style={{ borderRadius: 6 }} />
                  </div>
                </div>
              ))}
            </>
          )}

          {/* Error state */}
          {!loading && error && (
            <div className="bento-error-state span-12">
              <div className="bento-error-icon">
                <AlertCircle size={32} />
              </div>
              <p>{tw.loadFailed}</p>
              <button className="btn btn-solid-success" onClick={loadItems}>
                <RefreshCw size={16} />
                {tw.retry}
              </button>
            </div>
          )}

          {/* Content */}
          {!loading && !error && (
            <>
              {filteredItems.map((item) => (
                <RecycleCard
                  key={item.id}
                  item={item}
                  tw={tw}
                  onRestore={handleRestore}
                  onDelete={handleDeleteRequest}
                />
              ))}

              {filteredItems.length === 0 && (
                <RecycleEmptyState
                  tw={tw}
                  isFilterResult={(activeFilter !== 'all' || normalizedQuery.length > 0) && items.length > 0}
                />
              )}
            </>
          )}
        </div>
      </div>

      {/* Confirm dialog for destructive actions */}
      <ConfirmDialog
        open={confirmAction !== null}
        title={confirmAction?.type === 'emptyAll' ? tw.emptyAll : tw.deletePermanent}
        description={confirmAction?.type === 'emptyAll' ? tw.emptyConfirm : tw.deletePermanentConfirm}
        danger
        onConfirm={handleConfirm}
        onCancel={handleCancelConfirm}
      />
    </div>
  )
}