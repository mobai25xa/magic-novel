import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

function createCheck(name, pass, detail) {
  return detail ? { name, pass, detail } : { name, pass }
}

async function main() {
  const projectRoot = resolve(import.meta.dirname, '..')
  const tempDir = await mkdtemp(join(tmpdir(), 'session-v2-canary-rollout-'))

  try {
    const inputPath = join(tempDir, 'input.json')
    const outputPath = join(tempDir, 'report.json')

    const fixture = {
      sessions: [
        { session_id: 's-10-1', stage: 10, status: 'ok' },
        {
          session_id: 's-10-2',
          stage: 10,
          status: 'failed',
          failure_type: 'append_error',
          rollback_result: 'rolled_back',
          rollback_success: true,
        },
        { session_id: 's-50-1', stage: 50, status: 'ok' },
        {
          session_id: 's-50-2',
          stage: 50,
          status: 'failed',
          failure_type: 'hydrate_error',
          rollback_result: 'rollback_failed',
          rollback_success: false,
        },
        { session_id: 's-100-1', stage: 100, status: 'ok' },
      ],
    }

    await writeFile(inputPath, `${JSON.stringify(fixture, null, 2)}\n`, 'utf-8')

    const run = spawnSync(
      'node',
      [
        'scripts/session-v2-canary-rollout.mjs',
        '--input',
        inputPath,
        '--output',
        outputPath,
        '--max-failure-rate',
        '0.6',
        '--max-rollback-rate',
        '0.6',
      ],
      {
        cwd: projectRoot,
        shell: true,
        encoding: 'utf-8',
      },
    )

    const report = JSON.parse(await readFile(outputPath, 'utf-8'))
    const stage10 = report.stages.find((stage) => stage.stage === '10%')
    const failedSession = stage10?.failed_sessions?.find((item) => item.session_id === 's-10-2')

    const checks = [
      createCheck('rollout_executor_exit_success', run.status === 0, run.stderr || run.stdout),
      createCheck('report_contains_stages', Array.isArray(report.stages) && report.stages.length >= 3),
      createCheck(
        'report_contains_failure_session_id_type_and_rollback_result',
        Boolean(failedSession)
          && failedSession.failure_type === 'append_error'
          && failedSession.rollback_result === 'rolled_back'
          && failedSession.rollback_success === true,
      ),
      createCheck(
        'report_summary_counts',
        report.summary?.total_sessions === 5
          && report.summary?.failed_sessions === 2
          && report.summary?.rolled_back_sessions === 1,
      ),
    ]

    const allPass = checks.every((check) => check.pass)
    console.log('[test-session-canary-rollout]', JSON.stringify({ all_pass: allPass, checks }))

    if (!allPass) {
      process.exit(1)
    }
  } finally {
    await rm(tempDir, { recursive: true, force: true })
  }
}

main().catch((error) => {
  console.error('[test-session-canary-rollout] failed:', error.message)
  process.exit(1)
})
