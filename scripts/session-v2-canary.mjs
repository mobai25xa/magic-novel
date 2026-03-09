import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function parseArgs(argv) {
  const args = {
    report: '',
    maxFailureRate: 0.05,
    maxRollbackRate: 0.02,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    const value = argv[i + 1]

    if (token === '--report' && value) {
      args.report = value
      i += 1
      continue
    }

    if (token === '--max-failure-rate' && value) {
      args.maxFailureRate = Number(value)
      i += 1
      continue
    }

    if (token === '--max-rollback-rate' && value) {
      args.maxRollbackRate = Number(value)
      i += 1
    }
  }

  return args
}

function asNumber(value) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value
  }

  if (typeof value === 'string' && value.trim()) {
    const parsed = Number(value)
    if (Number.isFinite(parsed)) {
      return parsed
    }
  }

  return undefined
}

function normalizeStage(raw, index) {
  const total = asNumber(raw.total)
    ?? asNumber(raw.total_sessions)
    ?? asNumber(raw.sessions_total)
    ?? 0
  const failed = asNumber(raw.failed)
    ?? asNumber(raw.failed_sessions)
    ?? asNumber(raw.sessions_failed)
    ?? 0
  const rolledBack = asNumber(raw.rolled_back)
    ?? asNumber(raw.rollback_sessions)
    ?? asNumber(raw.sessions_rolled_back)
    ?? 0

  const stage = String(raw.stage ?? raw.name ?? raw.weight ?? `${index + 1}`).trim() || `${index + 1}`

  const failureRate = total > 0 ? failed / total : 0
  const rollbackRate = total > 0 ? rolledBack / total : 0

  return {
    stage,
    total,
    failed,
    rolledBack,
    failureRate,
    rollbackRate,
  }
}

async function loadReport(path) {
  const raw = await readFile(path, 'utf-8')
  const parsed = JSON.parse(raw)

  const list = Array.isArray(parsed.stages)
    ? parsed.stages
    : Array.isArray(parsed.windows)
      ? parsed.windows
      : Array.isArray(parsed)
        ? parsed
        : []

  return list.map(normalizeStage)
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  const reportPath = args.report
    ? resolve(process.cwd(), args.report)
    : resolve(process.cwd(), 'session-v2-canary-report.json')

  const stages = await loadReport(reportPath)
  if (stages.length === 0) {
    throw new Error(`No canary stages found in report: ${reportPath}`)
  }

  const checks = stages.map((stage) => ({
    stage: stage.stage,
    total: stage.total,
    failed: stage.failed,
    rolled_back: stage.rolledBack,
    failure_rate: Number(stage.failureRate.toFixed(6)),
    rollback_rate: Number(stage.rollbackRate.toFixed(6)),
    pass:
      stage.failureRate <= args.maxFailureRate
      && stage.rollbackRate <= args.maxRollbackRate,
  }))

  const allPass = checks.every((item) => item.pass)
  console.log('[session-v2-canary]', JSON.stringify({
    all_pass: allPass,
    thresholds: {
      max_failure_rate: args.maxFailureRate,
      max_rollback_rate: args.maxRollbackRate,
    },
    checks,
  }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[session-v2-canary] failed:', error.message)
  process.exit(1)
})
