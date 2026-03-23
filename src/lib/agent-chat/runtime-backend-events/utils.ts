export function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as Record<string, unknown>
}

export function extractToolExposureMeta(payload: Record<string, unknown>) {
  const policySource = typeof payload.policy_source === 'string' ? payload.policy_source : undefined
  const capabilityPreset = typeof payload.capability_preset === 'string' ? payload.capability_preset : undefined
  const exposureReason = typeof payload.exposure_reason === 'string' ? payload.exposure_reason : undefined
  const toolCallCount = typeof payload.tool_call_count === 'number' ? payload.tool_call_count : undefined
  const roundsExecuted = typeof payload.rounds_executed === 'number' ? payload.rounds_executed : undefined
  const exposedTools = Array.isArray(payload.exposed_tools)
    ? payload.exposed_tools.filter((value): value is string => typeof value === 'string')
    : []
  const skippedTools = Array.isArray(payload.skipped_tools)
    ? payload.skipped_tools
      .map((value) => asRecord(value))
      .filter((value): value is Record<string, unknown> => Boolean(value))
    : []

  if (
    !policySource
    && !capabilityPreset
    && !exposureReason
    && toolCallCount === undefined
    && roundsExecuted === undefined
    && exposedTools.length === 0
    && skippedTools.length === 0
  ) {
    return undefined
  }

  return {
    policy_source: policySource,
    capability_preset: capabilityPreset,
    exposure_reason: exposureReason,
    tool_call_count: toolCallCount,
    rounds_executed: roundsExecuted,
    exposed_tools: exposedTools,
    skipped_tools: skippedTools,
  }
}

export function buildToolExposureSummary(payload: Record<string, unknown>) {
  const capabilityPreset = typeof payload.capability_preset === 'string'
    ? payload.capability_preset
    : typeof payload.policy_source === 'string'
      ? payload.policy_source
      : 'unknown'
  const exposedTools = Array.isArray(payload.exposed_tools)
    ? payload.exposed_tools.filter((value): value is string => typeof value === 'string')
    : []
  const exposureReason = typeof payload.exposure_reason === 'string' ? payload.exposure_reason : undefined
  const base = `capability · ${capabilityPreset} (${exposedTools.length})`
  return exposureReason ? `${base} · ${exposureReason}` : base
}

export function parseArgsPreview(raw: unknown): Record<string, unknown> {
  if (!raw) return {}
  const direct = asRecord(raw)
  if (direct) return direct
  if (typeof raw === 'string') {
    try {
      const parsed = JSON.parse(raw)
      return asRecord(parsed) || { _raw: raw }
    } catch {
      return { _raw: raw }
    }
  }
  return {}
}

export function toToolTraceStage(raw: unknown): 'policy' | 'execute' | 'result' | undefined {
  switch (raw) {
    case 'policy':
    case 'execute':
    case 'result':
      return raw
    default:
      return undefined
  }
}

export function normalizeStopReason(
  raw: unknown,
): 'success' | 'cancel' | 'error' | 'limit' | undefined {
  const s = String(raw ?? '')
  switch (s) {
    case 'success':
    case 'cancel':
    case 'error':
    case 'limit':
      return s
    case 'waiting_confirmation':
      return 'cancel'
    case 'waiting_askuser':
      return 'cancel'
    default:
      return 'success'
  }
}
