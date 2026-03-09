import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, relative, resolve } from 'node:path'

const projectRoot = resolve(import.meta.dirname, '..')
const workspaceRoot = resolve(projectRoot, '..')
const defaultOutput = resolve(
  workspaceRoot,
  'docs',
  'magic_plan',
  'plan_reconstruction',
  '_artifacts',
  'session-v2-canary-rollout-report.json',
)

function parseArgs(argv) {
  const args = {
    input: '',
    output: defaultOutput,
    stages: [10, 50, 100],
    maxFailureRate: 0.05,
    maxRollbackRate: 0.02,
  }

  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i]
    const value = argv[i + 1]

    if (token === '--input' && value) {
      args.input = value
      i += 1
      continue
    }

    if (token === '--output' && value) {
      args.output = value
      i += 1
      continue
    }

    if (token === '--stages' && value) {
      args.stages = parseStages(value)
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

function parseStages(value) {
  const parsed = String(value)
    .split(',')
    .map((item) => Number(item.trim()))
    .filter((item) => Number.isFinite(item) && item > 0 && item <= 100)

  if (parsed.length === 0) {
    return [10, 50, 100]
  }

  return [...new Set(parsed)].sort((a, b) => a - b)
}

function asText(value) {
  if (typeof value !== 'string') {
    return undefined
  }
  const normalized = value.trim()
  return normalized || undefined
}

function asBoolean(value) {
  if (typeof value === 'boolean') {
    return value
  }

  if (typeof value === 'number') {
    return value !== 0
  }

  if (typeof value === 'string') {
    const normalized = value.trim().toLowerCase()
    if (!normalized) {
      return undefined
    }
    if (['1', 'true', 'yes', 'y'].includes(normalized)) {
      return true
    }
    if (['0', 'false', 'no', 'n'].includes(normalized)) {
      return false
    }
  }

  return undefined
}

function resolveStageLabel(raw, index, total, configuredStages) {
  const direct = raw.stage ?? raw.rollout_stage ?? raw.weight ?? raw.window
  if (direct !== undefined && direct !== null) {
    const numeric = Number(direct)
    if (Number.isFinite(numeric) && numeric > 0) {
      return `${numeric}%`
    }

    const text = asText(direct)
    if (text) {
      return text
    }
  }

  const progress = ((index + 1) / Math.max(1, total)) * 100
  const stage = configuredStages.find((value) => progress <= value) ?? configuredStages[configuredStages.length - 1]
  return `${stage}%`
}

function normalizeSession(raw, index, total, configuredStages) {
  const sessionId = asText(raw.session_id)
    || asText(raw.sessionId)
    || asText(raw.id)
    || `session_${index + 1}`

  const status = asText(raw.status)?.toLowerCase()
  const explicitFailed = asBoolean(raw.failed)
  const failureType = asText(raw.failure_type)
    || asText(raw.failureType)
    || asText(raw.error_type)
    || asText(raw.errorType)
    || asText(raw.error_code)
    || asText(raw.errorCode)

  const failed = typeof explicitFailed === 'boolean'
    ? explicitFailed
    : status === 'failed' || status === 'error' || Boolean(failureType)

  const rollbackSuccess = asBoolean(raw.rollback_success)
    ?? asBoolean(raw.rollbackSuccess)
    ?? asBoolean(raw.rolled_back)
    ?? asBoolean(raw.rolledBack)

  const rollbackResult = asText(raw.rollback_result)
    || asText(raw.rollbackResult)
    || asText(raw.rollback_status)
    || asText(raw.rollbackStatus)
    || (failed ? (rollbackSuccess ? 'rolled_back' : 'not_rolled_back') : 'not_required')

  return {
    session_id: sessionId,
    stage: resolveStageLabel(raw, index, total, configuredStages),
    failed,
    failure_type: failed ? (failureType || 'unknown_failure') : undefined,
    rollback_success: failed ? Boolean(rollbackSuccess) : false,
    rollback_result: failed ? rollbackResult : 'not_required',
  }
}

async function loadSessions(inputPath) {
  const raw = await readFile(inputPath, 'utf-8')
  const parsed = JSON.parse(raw)

  if (Array.isArray(parsed)) {
    return parsed
  }

  if (Array.isArray(parsed.sessions)) {
    return parsed.sessions
  }

  throw new Error(`Invalid canary input payload: ${inputPath}`)
}

function aggregateByStage(sessions, args) {
  const stageMap = new Map()
  const configuredOrder = args.stages.map((stage) => `${stage}%`)

  for (const session of sessions) {
    const key = session.stage
    if (!stageMap.has(key)) {
      stageMap.set(key, {
        stage: key,
        total: 0,
        failed: 0,
        rolled_back: 0,
        failed_sessions: [],
      })
    }

    const bucket = stageMap.get(key)
    bucket.total += 1
    if (session.failed) {
      bucket.failed += 1
      if (session.rollback_success) {
        bucket.rolled_back += 1
      }
      bucket.failed_sessions.push({
        session_id: session.session_id,
        failure_type: session.failure_type || 'unknown_failure',
        rollback_result: session.rollback_result,
        rollback_success: session.rollback_success,
      })
    }
  }

  const orderedStageKeys = [
    ...configuredOrder.filter((label) => stageMap.has(label)),
    ...[...stageMap.keys()].filter((label) => !configuredOrder.includes(label)).sort(),
  ]

  return orderedStageKeys.map((stageKey) => {
    const bucket = stageMap.get(stageKey)
    const failureRate = bucket.total > 0 ? bucket.failed / bucket.total : 0
    const rollbackRate = bucket.total > 0 ? bucket.rolled_back / bucket.total : 0

    return {
      ...bucket,
      failure_rate: Number(failureRate.toFixed(6)),
      rollback_rate: Number(rollbackRate.toFixed(6)),
      pass: failureRate <= args.maxFailureRate && rollbackRate <= args.maxRollbackRate,
    }
  })
}

function toWorkspacePath(absPath) {
  return relative(workspaceRoot, absPath).replace(/\\/g, '/')
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  if (!args.input) {
    throw new Error('Missing --input for canary rollout execution')
  }

  const inputPath = resolve(process.cwd(), args.input)
  const outputPath = resolve(process.cwd(), args.output)
  const rawSessions = await loadSessions(inputPath)
  if (rawSessions.length === 0) {
    throw new Error(`No session samples in input: ${inputPath}`)
  }

  const sessions = rawSessions.map((item, index) => normalizeSession(item, index, rawSessions.length, args.stages))
  const stages = aggregateByStage(sessions, args)

  const summary = {
    all_pass: stages.every((stage) => stage.pass),
    total_sessions: sessions.length,
    failed_sessions: sessions.filter((session) => session.failed).length,
    rolled_back_sessions: sessions.filter((session) => session.failed && session.rollback_success).length,
  }

  const report = {
    generated_at: new Date().toISOString(),
    input: toWorkspacePath(inputPath),
    thresholds: {
      max_failure_rate: args.maxFailureRate,
      max_rollback_rate: args.maxRollbackRate,
    },
    stages,
    summary,
  }

  await mkdir(dirname(outputPath), { recursive: true })
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf-8')

  console.log('[session-v2-canary-rollout]', JSON.stringify({
    all_pass: summary.all_pass,
    total_sessions: summary.total_sessions,
    failed_sessions: summary.failed_sessions,
    rolled_back_sessions: summary.rolled_back_sessions,
    report: toWorkspacePath(outputPath),
  }))

  if (!summary.all_pass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[session-v2-canary-rollout] failed:', error.message)
  process.exit(1)
})
