import assert from 'node:assert/strict'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { createTsModuleLoader } from './_ts-test-loader.mjs'

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const { loadModule } = createTsModuleLoader(rootDir)

const workflowHelpers = loadModule(resolve(rootDir, 'src/components/create/workflow-helpers.ts'))
const bootstrapHelpers = loadModule(resolve(rootDir, 'src/features/project-home/bootstrap-status-helpers.ts'))

assert.equal(
  workflowHelpers.DEFAULT_CREATE_PROJECT_TARGET_REF,
  'knowledge:.magic_novel/planning/story_blueprint.md',
)

assert.equal(
  workflowHelpers.resolveBootstrapRecommendedTargetRef('wait_for_bootstrap'),
  'knowledge:.magic_novel/planning/story_blueprint.md',
)
assert.equal(
  workflowHelpers.resolveBootstrapRecommendedTargetRef('start_chapter_one'),
  'knowledge:.magic_novel/planning/chapter_backlog.md',
)
assert.equal(
  workflowHelpers.resolveCreateProjectTargetRef({
    bootstrapStatus: null,
    bootstrapError: null,
    bootstrapUnsupported: false,
  }),
  workflowHelpers.DEFAULT_CREATE_PROJECT_TARGET_REF,
)
assert.equal(
  workflowHelpers.resolveCreateProjectTargetRef({
    bootstrapStatus: {
      phase: 'ready_to_write',
      recommended_next_action: 'start_chapter_one',
    },
    bootstrapError: null,
    bootstrapUnsupported: false,
  }),
  'knowledge:.magic_novel/planning/chapter_backlog.md',
)

assert.equal(
  bootstrapHelpers.shouldSyncBootstrapStatus({
    projectPath: 'D:/novel',
    projectBootstrapState: 'scaffold_ready',
    bootstrapStatus: null,
    bootstrapStatusProjectPath: null,
  }),
  false,
)
assert.equal(
  bootstrapHelpers.shouldSyncBootstrapStatus({
    projectPath: 'D:/novel',
    projectBootstrapState: 'ready_to_write',
    bootstrapStatus: null,
    bootstrapStatusProjectPath: null,
  }),
  true,
)
assert.equal(
  bootstrapHelpers.shouldSyncBootstrapStatus({
    projectPath: 'D:/novel',
    projectBootstrapState: 'scaffold_ready',
    bootstrapStatus: {
      phase: 'llm_generating',
    },
    bootstrapStatusProjectPath: 'D:/novel',
  }),
  true,
)

console.log('dev-e legacy bootstrap cleanup checks passed')
