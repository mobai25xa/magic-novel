import { useCallback, useEffect, useRef, useState } from 'react'

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
  const lastBundleIdRef = useRef<string | null>(null)
  const [acceptedByItemId, setAcceptedByItemId] = useState<Record<string, boolean>>({})

  useEffect(() => {
    if (bundleId === lastBundleIdRef.current) {
      return
    }

    lastBundleIdRef.current = bundleId
    if (!bundleId) {
      setAcceptedByItemId({})
      return
    }
    setAcceptedByItemId(buildAcceptedMap(items, (policy) => policy === 'auto_if_pass'))
  }, [bundleId, items])

  const onToggle = useCallback((item: { item_id: string; accept_policy?: unknown }) => {
    const policy = getAcceptPolicy(item)
    const canToggleToAccept = policy !== 'orchestrator_only'

    setAcceptedByItemId((prev) => {
      const checked = Boolean(prev[item.item_id])
      if (!canToggleToAccept && !checked) {
        return prev
      }
      return { ...prev, [item.item_id]: !prev[item.item_id] }
    })
  }, [])

  const onAcceptSafe = useCallback(() => {
    setAcceptedByItemId(buildAcceptedMap(items, (policy) => policy === 'auto_if_pass'))
  }, [items])

  const onAcceptAll = useCallback(() => {
    setAcceptedByItemId(buildAcceptedMap(items, (policy) => policy !== 'orchestrator_only'))
  }, [items])

  const onRejectAll = useCallback(() => {
    setAcceptedByItemId(buildAcceptedMap(items, () => false))
  }, [items])

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
