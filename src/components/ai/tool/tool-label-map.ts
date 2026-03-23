type ToolLabelMap = Record<string, string | undefined>

function normalizeToolName(toolName: unknown) {
  return String(toolName ?? '').trim()
}

export function resolveToolLabel(input: {
  toolName: string
  labels?: ToolLabelMap
  fallback?: string
}) {
  const normalized = normalizeToolName(input.toolName)
  if (!normalized) {
    return input.fallback || 'tool'
  }

  const label = input.labels?.[normalized]
  if (typeof label === 'string' && label.trim()) {
    return label
  }

  return input.fallback || normalized
}
