import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, relative, resolve } from 'node:path'
import { spawn } from 'node:child_process'

const projectRoot = resolve(import.meta.dirname, '..')
const workspaceRoot = resolve(projectRoot, '..')
const artifactsDir = resolve(workspaceRoot, 'docs', 'magic_works', 'wokes', '_artifacts')
const reportPath = resolve(artifactsDir, 'tool-contract-governance-report.json')
const regressionsOnly = process.argv.includes('--regressions-only')

const allSteps = [
  {
    name: 'schema-inventory-snapshot',
    category: 'contract_consistency',
    command: 'pnpm',
    args: ['run', 'test:tool-schema-inventory'],
    cwd: projectRoot,
  },
  {
    name: 'schema-timeout-contract',
    category: 'contract_consistency',
    command: 'cargo',
    args: [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      'agent_tools::registry::tests::test_tool_schemas_do_not_expose_unimplemented_per_call_timeout_ms',
      '--',
      '--exact',
    ],
    cwd: projectRoot,
  },
  {
    name: 'schema-structure-capability-contract',
    category: 'contract_consistency',
    command: 'cargo',
    args: [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      'agent_tools::registry::tests::test_structure_edit_schema_hides_unimplemented_knowledge_item_node_type',
      '--',
      '--exact',
    ],
    cwd: projectRoot,
  },
  {
    name: 'askuser-top-level-contract',
    category: 'contract_consistency',
    command: 'cargo',
    args: [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      'agent_engine::tool_formatters::askuser::tests::askuser_parser_allowlist_matches_registered_schema_properties',
      '--',
      '--exact',
    ],
    cwd: projectRoot,
  },
  {
    name: 'askuser-nested-contract',
    category: 'contract_consistency',
    command: 'cargo',
    args: [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      'agent_engine::tool_formatters::askuser::tests::askuser_nested_question_contract_matches_registered_schema',
      '--',
      '--exact',
    ],
    cwd: projectRoot,
  },
  {
    name: 'critical-write-tool-regressions',
    category: 'tool_contract_regression',
    command: 'cargo',
    args: [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      'agent_tools::runtime::contract_regressions',
    ],
    cwd: projectRoot,
  },
  {
    name: 'tool-stream-invalid-json-regression',
    category: 'tool_stream_regression',
    command: 'cargo',
    args: [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      'llm::accumulator::tests::test_invalid_final_tool_json_returns_stream_tool_args_error',
      '--',
      '--exact',
    ],
    cwd: projectRoot,
  },
  {
    name: 'tool-stream-top-level-shape-regression',
    category: 'tool_stream_regression',
    command: 'cargo',
    args: [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      'llm::accumulator::tests::test_non_object_tool_args_return_stream_tool_args_error',
      '--',
      '--exact',
    ],
    cwd: projectRoot,
  },
]

async function main() {
  const steps = regressionsOnly
    ? allSteps.filter((step) => step.category !== 'contract_consistency')
    : allSteps

  const results = []
  for (const step of steps) {
    const result = await runStep(step)
    results.push(result)
    printStepResult(result)
  }

  const summary = buildSummary(results)
  const report = {
    generated_at: new Date().toISOString(),
    regressions_only: regressionsOnly,
    pass: results.every((result) => result.pass),
    summary,
    steps: results,
  }

  await mkdir(dirname(reportPath), { recursive: true })
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf-8')

  console.log('[tool-contract-governance]', JSON.stringify({
    pass: report.pass,
    regressions_only: regressionsOnly,
    failed_steps: results.filter((result) => !result.pass).map((result) => result.name),
    report: toWorkspacePath(reportPath),
  }))

  if (!report.pass) {
    process.exit(1)
  }
}

function buildSummary(results) {
  const byCategory = {}
  for (const result of results) {
    const bucket = byCategory[result.category] || {
      total: 0,
      passed: 0,
      failed: 0,
      failed_steps: [],
    }
    bucket.total += 1
    if (result.pass) {
      bucket.passed += 1
    } else {
      bucket.failed += 1
      bucket.failed_steps.push(result.name)
    }
    byCategory[result.category] = bucket
  }

  return {
    total_steps: results.length,
    failed_steps: results.filter((result) => !result.pass).map((result) => result.name),
    by_category: byCategory,
  }
}

function runStep(step) {
  const startedAt = Date.now()
  return new Promise((resolveResult) => {
    const spawned = resolveSpawn(step.command, step.args)
    const child = spawn(spawned.command, spawned.args, {
      cwd: step.cwd,
      shell: false,
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

    child.on('error', (error) => {
      resolveResult({
        name: step.name,
        category: step.category,
        cwd: toProjectPath(step.cwd),
        command: `${step.command} ${step.args.join(' ')}`,
        pass: false,
        exit_code: null,
        duration_ms: Date.now() - startedAt,
        stdout: trimOutput(stdout),
        stderr: trimOutput(`${stderr}\n${error.message}`),
      })
    })

    child.on('close', (code) => {
      resolveResult({
        name: step.name,
        category: step.category,
        cwd: toProjectPath(step.cwd),
        command: `${step.command} ${step.args.join(' ')}`,
        pass: code === 0,
        exit_code: code,
        duration_ms: Date.now() - startedAt,
        stdout: trimOutput(stdout),
        stderr: trimOutput(stderr),
      })
    })
  })
}

function resolveSpawn(command, args) {
  if (process.platform === 'win32') {
    const escaped = [command, ...args.map(quoteWindowsArg)].join(' ')
    return {
      command: process.env.ComSpec || 'cmd.exe',
      args: ['/d', '/s', '/c', escaped],
    }
  }

  return { command, args }
}

function quoteWindowsArg(arg) {
  if (/[\s"]/u.test(arg)) {
    return `"${arg.replace(/"/g, '\\"')}"`
  }
  return arg
}

function printStepResult(result) {
  const icon = result.pass ? 'OK' : 'FAIL'
  console.log(`[tool-contract-governance][${icon}] ${result.name} (${result.duration_ms}ms)`)
}

function trimOutput(output) {
  const normalized = output.replace(/\r\n/g, '\n').trim()
  if (normalized.length <= 4000) return normalized
  return `${normalized.slice(0, 4000)}\n...[truncated]`
}

function toProjectPath(abs) {
  return relative(projectRoot, abs).replace(/\\/g, '/') || '.'
}

function toWorkspacePath(abs) {
  return relative(workspaceRoot, abs).replace(/\\/g, '/')
}

main().catch((error) => {
  console.error('[tool-contract-governance] failed:', error.message)
  process.exit(1)
})
