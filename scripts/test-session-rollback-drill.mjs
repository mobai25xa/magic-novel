import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

function createCheck(name, pass, detail) {
  return detail ? { name, pass, detail } : { name, pass }
}

async function main() {
  const projectRoot = resolve(import.meta.dirname, '..')
  const tempDir = await mkdtemp(join(tmpdir(), 'session-v2-rollback-drill-'))

  try {
    const inputPath = join(tempDir, 'rollback-drill-input.json')
    const outputPath = join(tempDir, 'rollback-drill-report.json')

    const fixture = {
      drills: [
        {
          drill_id: 'batch_drill_1',
          scope: 'batch',
          session_ids: ['session_a', 'session_b'],
          duration_ms: 14000,
          rollback_success: true,
        },
        {
          drill_id: 'project_drill_1',
          scope: 'project',
          session_ids: ['session_project_1'],
          duration_ms: 22000,
          rollback_success: true,
        },
      ],
    }

    await writeFile(inputPath, `${JSON.stringify(fixture, null, 2)}\n`, 'utf-8')

    const run = spawnSync(
      'node',
      [
        'scripts/session-v2-rollback-drill.mjs',
        '--input',
        inputPath,
        '--output',
        outputPath,
        '--max-duration-ms',
        '30000',
      ],
      {
        cwd: projectRoot,
        shell: true,
        encoding: 'utf-8',
      },
    )

    const report = JSON.parse(await readFile(outputPath, 'utf-8'))

    const checks = [
      createCheck('rollback_drill_executor_exit_success', run.status === 0, run.stderr || run.stdout),
      createCheck(
        'rollback_drill_requires_batch_and_project',
        report.summary?.has_batch_drill === true && report.summary?.has_project_drill === true,
      ),
      createCheck('rollback_drill_all_pass', report.summary?.all_pass === true),
      createCheck(
        'rollback_drill_report_contains_drill_details',
        Array.isArray(report.drills)
          && report.drills.length === 2
          && report.drills.every((item) => item.rollback_success === true),
      ),
    ]

    const allPass = checks.every((check) => check.pass)
    console.log('[test-session-rollback-drill]', JSON.stringify({ all_pass: allPass, checks }))

    if (!allPass) {
      process.exit(1)
    }
  } finally {
    await rm(tempDir, { recursive: true, force: true })
  }
}

main().catch((error) => {
  console.error('[test-session-rollback-drill] failed:', error.message)
  process.exit(1)
})
