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
  'session-v2-rollback-drill-report.json',
)

function parseArgs(argv) {
  const args = {
    input: '',
    output: defaultOutput,
    maxDurationMs: 30000,
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

    if (token === '--max-duration-ms' && value) {
      args.maxDurationMs = Number(value)
      i += 1
    }
  }

  return args
}

function asText(value) {
  if (typeof value !== 'string') {
    return undefined
  }
  const normalized = value.trim()
  return normalized || undefined
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

function asBoolean(value) {
  if (typeof value === 'boolean') {
    return value
  }
  if (typeof value === 'number') {
    return value !== 0
  }
  if (typeof value === 'string') {
    const normalized = value.trim().toLowerCase()
    if (['1', 'true', 'yes', 'y'].includes(normalized)) {
      return true
    }
    if (['0', 'false', 'no', 'n'].includes(normalized)) {
      return false
    }
  }
  return undefined
}

function normalizeScope(raw) {
  const scope = asText(raw)?.toLowerCase()
  if (scope === 'batch' || scope === 'project') {
    return scope
  }
  return 'batch'
}

function normalizeSessionIds(raw) {
  if (Array.isArray(raw)) {
    return raw
      .map((item) => asText(item))
      .filter((item) => Boolean(item))
  }

  const single = asText(raw)
  return single ? [single] : []
}

function normalizeDrill(raw, index, maxDurationMs) {
  const scope = normalizeScope(raw.scope ?? raw.level)
  const sessionIds = normalizeSessionIds(raw.session_ids ?? raw.sessions ?? raw.session_id)
  const durationMs = asNumber(raw.duration_ms ?? raw.durationMs) ?? 0
  const rollbackSuccess = asBoolean(raw.rollback_success ?? raw.rollbackSuccess)
    ?? asBoolean(raw.success)
    ?? false
  const failureType = asText(raw.failure_type ?? raw.failureType)
    || (rollbackSuccess ? undefined : 'rollback_failed')

  const id = asText(raw.drill_id)
    || asText(raw.id)
    || `${scope}_drill_${index + 1}`

  const durationPass = durationMs <= maxDurationMs
  const pass = rollbackSuccess && durationPass

  return {
    drill_id: id,
    scope,
    session_ids: sessionIds,
    duration_ms: durationMs,
    rollback_success: rollbackSuccess,
    failure_type: failureType,
    pass,
  }
}

async function loadInput(path) {
  const raw = await readFile(path, 'utf-8')
  const parsed = JSON.parse(raw)

  if (Array.isArray(parsed)) {
    return parsed
  }
  if (Array.isArray(parsed.drills)) {
    return parsed.drills
  }

  throw new Error(`Invalid rollback drill input payload: ${path}`)
}

function summarize(drills) {
  const byScope = {
    batch: { total: 0, passed: 0 },
    project: { total: 0, passed: 0 },
  }

  for (const drill of drills) {
    byScope[drill.scope].total += 1
    if (drill.pass) {
      byScope[drill.scope].passed += 1
    }
  }

  return byScope
}

function toWorkspacePath(absPath) {
  return relative(workspaceRoot, absPath).replace(/\\/g, '/')
}

async function main() {
  const args = parseArgs(process.argv.slice(2))
  if (!args.input) {
    throw new Error('Missing --input for rollback drill execution')
  }

  const inputPath = resolve(process.cwd(), args.input)
  const outputPath = resolve(process.cwd(), args.output)

  const rawDrills = await loadInput(inputPath)
  if (rawDrills.length === 0) {
    throw new Error(`No rollback drills found in input: ${inputPath}`)
  }

  const drills = rawDrills.map((item, index) => normalizeDrill(item, index, args.maxDurationMs))
  const scopeSummary = summarize(drills)

  const hasBatchDrill = scopeSummary.batch.total > 0
  const hasProjectDrill = scopeSummary.project.total > 0
  const failedDrills = drills.filter((drill) => !drill.pass)

  const allPass = hasBatchDrill
    && hasProjectDrill
    && failedDrills.length === 0

  const report = {
    generated_at: new Date().toISOString(),
    input: toWorkspacePath(inputPath),
    thresholds: {
      max_duration_ms: args.maxDurationMs,
      requires_scopes: ['batch', 'project'],
    },
    summary: {
      all_pass: allPass,
      total_drills: drills.length,
      failed_drills: failedDrills.length,
      has_batch_drill: hasBatchDrill,
      has_project_drill: hasProjectDrill,
      scope: scopeSummary,
    },
    drills,
    failed_drill_details: failedDrills,
  }

  await mkdir(dirname(outputPath), { recursive: true })
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf-8')

  console.log('[session-v2-rollback-drill]', JSON.stringify({
    all_pass: report.summary.all_pass,
    total_drills: report.summary.total_drills,
    failed_drills: report.summary.failed_drills,
    has_batch_drill: report.summary.has_batch_drill,
    has_project_drill: report.summary.has_project_drill,
    report: toWorkspacePath(outputPath),
  }))

  if (!report.summary.all_pass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[session-v2-rollback-drill] failed:', error.message)
  process.exit(1)
})
