import { readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve, relative } from 'node:path'
import { spawn } from 'node:child_process'
import { mkdir } from 'node:fs/promises'

const projectRoot = resolve(import.meta.dirname, '..')
const workspaceRoot = resolve(projectRoot, '..')
const artifactsDir = resolve(workspaceRoot, 'docs', 'magic_plan', 'plan_reconstruction', '_artifacts')
const reportPath = resolve(artifactsDir, 'phase6-dod-report.json')

const strict = process.argv.includes('--strict')

async function main() {
  const steps = [
    { name: 'lint', command: 'pnpm', args: ['run', 'lint'], cwd: projectRoot },
    { name: 'build', command: 'pnpm', args: ['run', 'build'], cwd: projectRoot },
    { name: 'agent-chat-check', command: 'pnpm', args: ['run', 'test:agent-chat'], cwd: projectRoot },
    { name: 'tool-agent-check', command: 'pnpm', args: ['run', 'test:tool-agent'], cwd: projectRoot },
    { name: 'tool-agent-scenarios', command: 'pnpm', args: ['run', 'test:tool-agent-scenarios'], cwd: projectRoot },
    { name: 'tool-agent-stability', command: 'pnpm', args: ['run', 'test:tool-agent-stability'], cwd: projectRoot },
    { name: 'governance', command: 'node', args: ['scripts/check-governance.mjs', ...(strict ? ['--strict'] : [])], cwd: projectRoot },
    { name: 'cargo-test', command: 'cargo', args: ['test'], cwd: resolve(projectRoot, 'src-tauri') },
  ]

  const results = []
  for (const step of steps) {
    const result = await runStep(step)
    results.push(result)
    printStepResult(result)
  }

  const governanceReport = await loadJsonSafe(resolve(artifactsDir, 'phase6-governance-report.json'))

  const summary = {
    strict,
    generated_at: new Date().toISOString(),
    pass: results.every((r) => r.pass),
    failed_steps: results.filter((r) => !r.pass).map((r) => r.name),
  }

  const report = {
    ...summary,
    steps: results,
    governance_summary: governanceReport?.summary || null,
  }

  await mkdir(dirname(reportPath), { recursive: true })
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf-8')

  console.log('[phase6-dod]', JSON.stringify({
    pass: report.pass,
    strict,
    failed_steps: report.failed_steps,
    report: toWorkspacePath(reportPath),
  }))

  if (!report.pass) {
    process.exit(1)
  }
}

function runStep(step) {
  const startedAt = Date.now()
  return new Promise((resolveResult) => {
    const child = spawn(step.command, step.args, {
      cwd: step.cwd,
      shell: true,
      env: process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    })

    let stdout = ''
    let stderr = ''

    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString()
    })
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString()
    })

    child.on('close', (code) => {
      const durationMs = Date.now() - startedAt
      resolveResult({
        name: step.name,
        cwd: toProjectPath(step.cwd),
        command: `${step.command} ${step.args.join(' ')}`,
        pass: code === 0,
        exit_code: code,
        duration_ms: durationMs,
        stdout: trimOutput(stdout),
        stderr: trimOutput(stderr),
      })
    })
  })
}

function printStepResult(result) {
  const icon = result.pass ? 'OK' : 'FAIL'
  console.log(`[phase6-dod][${icon}] ${result.name} (${result.duration_ms}ms)`)
}

function trimOutput(output) {
  const normalized = output.replace(/\r\n/g, '\n').trim()
  if (normalized.length <= 4000) return normalized
  return `${normalized.slice(0, 4000)}\n...[truncated]`
}

async function loadJsonSafe(path) {
  try {
    const raw = await readFile(path, 'utf-8')
    return JSON.parse(raw)
  } catch {
    return null
  }
}

function toProjectPath(abs) {
  return relative(projectRoot, abs).replace(/\\/g, '/') || '.'
}

function toWorkspacePath(abs) {
  return relative(workspaceRoot, abs).replace(/\\/g, '/')
}

main().catch((error) => {
  console.error('[phase6-dod] failed:', error.message)
  process.exit(1)
})
