import { randomUUID } from 'node:crypto'

function classifyFaultDomain(code) {
  if (code.startsWith('E_VC_')) return 'vc'
  if (code.startsWith('E_JVM_')) return 'jvm'
  return 'tool'
}

function normalizeToolError(errorCode) {
  return {
    code: errorCode,
    fault_domain: classifyFaultDomain(errorCode),
  }
}

function createCallId() {
  return `call_${Date.now()}_${randomUUID().slice(0, 8)}`
}

function simulateOne(i) {
  const phase = i % 10

  if (phase < 7) {
    return {
      ok: true,
      call_id: createCallId(),
      tool: phase < 2 ? 'create' : phase < 4 ? 'read' : 'edit',
      fault_domain: null,
    }
  }

  if (phase === 7) {
    const err = normalizeToolError('E_JVM_PARSE_FAILED')
    return { ok: false, call_id: createCallId(), tool: 'edit', fault_domain: err.fault_domain }
  }

  if (phase === 8) {
    const err = normalizeToolError('E_VC_CONFLICT_REVISION')
    return { ok: false, call_id: createCallId(), tool: 'edit', fault_domain: err.fault_domain }
  }

  const err = normalizeToolError('E_TOOL_SCHEMA_INVALID')
  return { ok: false, call_id: createCallId(), tool: 'create', fault_domain: err.fault_domain }
}

async function main() {
  const runs = 120
  const results = []

  for (let i = 0; i < runs; i += 1) {
    results.push(simulateOne(i))
  }

  const uniqueCallIds = new Set(results.map((r) => r.call_id)).size
  const success = results.filter((r) => r.ok).length
  const failed = results.length - success

  const byDomain = results
    .filter((r) => !r.ok)
    .reduce((acc, r) => {
      const key = r.fault_domain || 'unknown'
      acc[key] = (acc[key] || 0) + 1
      return acc
    }, {})

  const summary = {
    runs,
    success,
    failed,
    unique_call_ids: uniqueCallIds,
    call_id_collision: uniqueCallIds !== runs,
    failure_by_domain: byDomain,
  }

  console.log('[tool-agent-stability]', JSON.stringify(summary))

  if (summary.call_id_collision) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[tool-agent-stability] failed:', error.message)
  process.exit(1)
})
