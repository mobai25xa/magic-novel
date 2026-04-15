import { useCallback, useMemo, useState } from 'react'

import { missionKnowledgeApplyFeature, missionKnowledgeDecideFeature, missionKnowledgeRollbackFeature } from '@/features/agent-chat'

import type { KnowledgeLatestPayload } from '../types'

function getAcceptPolicy(item: { accept_policy?: unknown }) {
  return String(item.accept_policy ?? '').trim()
}

function buildAcceptedMap(items: Array<{ item_id: string; accept_policy?: unknown }>, predicate: (policy: string) => boolean) {
  const next: Record<string, boolean> = {}
  for (const item of items) {
    next[item.item_id] = predicate(getAcceptPolicy(item))
  }
  return next
}

function useActionState() {
  const [actionLoading, setActionLoading] = useState(false)
  const [actionError, setActionError] = useState<string | null>(null)

  const run = useCallback(async (fn: () => Promise<void>) => {
    setActionError(null)
    setActionLoading(true)
    try {
      await fn()
    } catch (e) {
      setActionError(String(e))
    } finally {
      setActionLoading(false)
    }
  }, [])

  return { actionLoading, actionError, setActionError, run }
}

function buildDecisionPayload(input: {
  knowledgeLatest: KnowledgeLatestPayload | null
  acceptedByItemId: Record<string, boolean>
}): { error?: string; payload?: { schema_version: 1; actor: 'user'; bundle_id: string; delta_id: string; accepted_item_ids: string[]; rejected_item_ids: string[] } } {
  const bundle = input.knowledgeLatest?.bundle ?? null
  if (!bundle || !Array.isArray(bundle.proposal_items) || bundle.proposal_items.length === 0) {
    return { error: 'No knowledge proposals available to decide' }
  }

  const deltaId = input.knowledgeLatest?.delta?.knowledge_delta_id
  if (!deltaId) {
    return { error: 'Missing knowledge delta_id for decision' }
  }

  const accepted_item_ids = bundle.proposal_items
    .filter((item) => Boolean(input.acceptedByItemId[item.item_id]))
    .map((item) => item.item_id)
  const rejected_item_ids = bundle.proposal_items
    .filter((item) => !input.acceptedByItemId[item.item_id])
    .map((item) => item.item_id)

  return {
    payload: {
      schema_version: 1,
      actor: 'user',
      bundle_id: bundle.bundle_id,
      delta_id: deltaId,
      accepted_item_ids,
      rejected_item_ids,
    },
  }
}

export function deriveKnowledgeSummary(knowledgeLatest: KnowledgeLatestPayload | null, knowledgeDecisionRequired: boolean) {
  const bundle = knowledgeLatest?.bundle ?? null
  const delta = knowledgeLatest?.delta ?? null

  const proposalItems = Array.isArray(bundle?.proposal_items) ? bundle!.proposal_items : []
  const conflictCount = delta?.conflicts?.length ?? 0
  const proposalCount = proposalItems.length
  const acceptedCount = delta?.accepted_item_ids?.length ?? 0
  const rejectedCount = delta?.rejected_item_ids?.length ?? 0
  const statusLabel = delta?.status ?? (bundle ? 'proposed' : 'empty')

  return {
    bundle,
    delta,
    proposalItems,
    conflictCount,
    proposalCount,
    acceptedCount,
    rejectedCount,
    statusLabel,
    defaultOpen: conflictCount > 0 || knowledgeDecisionRequired,
    canDecide: Boolean(bundle && proposalCount > 0),
    canApply: delta?.status === 'accepted' && conflictCount === 0,
    canRollback: delta?.status === 'applied',
  }
}

export function useKnowledgeProposalSelection(input: {
  bundleId: string | null
  items: Array<{ item_id: string; accept_policy?: unknown }>
}) {
  const { bundleId, items } = input
  const [selectionState, setSelectionState] = useState<{
    bundleId: string | null
    acceptedByItemId: Record<string, boolean>
  }>({
    bundleId: null,
    acceptedByItemId: {},
  })
  const acceptedByItemId = useMemo(() => {
    if (!bundleId) {
      return {}
    }

    if (selectionState.bundleId !== bundleId) {
      return buildAcceptedMap(items, (policy) => policy === 'auto_if_pass')
    }

    return selectionState.acceptedByItemId
  }, [bundleId, items, selectionState.acceptedByItemId, selectionState.bundleId])

  const onToggle = useCallback((item: { item_id: string; accept_policy?: unknown }) => {
    const policy = getAcceptPolicy(item)
    const canToggleToAccept = policy !== 'orchestrator_only'

    setSelectionState((prev) => {
      const base = !bundleId
        ? {}
        : prev.bundleId === bundleId
          ? prev.acceptedByItemId
          : buildAcceptedMap(items, (nextPolicy) => nextPolicy === 'auto_if_pass')
      const checked = Boolean(base[item.item_id])
      if (!canToggleToAccept && !checked) {
        return prev
      }

      return {
        bundleId,
        acceptedByItemId: { ...base, [item.item_id]: !base[item.item_id] },
      }
    })
  }, [bundleId, items])

  const onAcceptSafe = useCallback(() => {
    setSelectionState({
      bundleId,
      acceptedByItemId: bundleId ? buildAcceptedMap(items, (policy) => policy === 'auto_if_pass') : {},
    })
  }, [bundleId, items])

  const onAcceptAll = useCallback(() => {
    setSelectionState({
      bundleId,
      acceptedByItemId: bundleId ? buildAcceptedMap(items, (policy) => policy !== 'orchestrator_only') : {},
    })
  }, [bundleId, items])

  const onRejectAll = useCallback(() => {
    setSelectionState({
      bundleId,
      acceptedByItemId: bundleId ? buildAcceptedMap(items, () => false) : {},
    })
  }, [bundleId, items])

  return { acceptedByItemId, onToggle, onAcceptSafe, onAcceptAll, onRejectAll }
}

export function useKnowledgeActions(input: {
  projectPath: string
  missionId: string
  knowledgeLatest: KnowledgeLatestPayload | null
  acceptedByItemId: Record<string, boolean>
  onRefresh: () => void
}) {
  const { projectPath, missionId, knowledgeLatest, acceptedByItemId, onRefresh } = input
  const { actionLoading, actionError, setActionError, run } = useActionState()

  const onDecide = useCallback(async () => {
    const built = buildDecisionPayload({ knowledgeLatest, acceptedByItemId })
    if (built.error) {
      setActionError(built.error)
      return
    }

    return run(async () => {
      await missionKnowledgeDecideFeature({
        project_path: projectPath,
        mission_id: missionId,
        decision: built.payload!,
      })
      onRefresh()
    })
  }, [acceptedByItemId, knowledgeLatest, missionId, onRefresh, projectPath, run, setActionError])

  const onApply = useCallback(async () => {
    return run(async () => {
      await missionKnowledgeApplyFeature(projectPath, missionId)
      onRefresh()
    })
  }, [missionId, onRefresh, projectPath, run])

  const onRollback = useCallback(async () => {
    if (!window.confirm('Rollback the latest knowledge apply?')) return

    return run(async () => {
      const token = knowledgeLatest?.delta?.rollback?.token
      await missionKnowledgeRollbackFeature(projectPath, missionId, token)
      onRefresh()
    })
  }, [knowledgeLatest?.delta?.rollback?.token, missionId, onRefresh, projectPath, run])

  return { actionLoading, actionError, onDecide, onApply, onRollback }
}
