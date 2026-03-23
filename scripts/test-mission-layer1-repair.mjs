import { resolve } from 'node:path'

import { createTsModuleLoader } from './_ts-test-loader.mjs'

function createCheck(name, pass, detail) {
  return detail ? { name, pass, detail } : { name, pass }
}

async function main() {
  const root = resolve(import.meta.dirname, '..')
  const loadTs = createTsModuleLoader(root)
  const helpers = loadTs.loadModule(resolve(root, 'src/components/ai/mission-panel/mission-panel-action-helpers.ts'))

  const originalNow = Date.now
  Date.now = () => 1700000000123

  try {
    const chapterCard = helpers.buildChapterCardDoc({
      existing: {
        schema_version: 3,
        scope_ref: 'chapter:legacy',
        workflow_kind: 'arc',
        hard_constraints: ['old'],
        success_criteria: ['old'],
        status: 'active',
        updated_at: 1,
      },
      scopeRef: 'chapter:vol-01:ch-02',
      scopeLocator: 'vol-01/ch-02.md',
      draft: {
        objective: '锁定当前章节目标',
        hard_constraints: ['保持第一人称不变'],
        success_criteria: ['完成冲突升级'],
      },
    })

    const recentFacts = helpers.buildRecentFactsDoc({
      existing: null,
      scopeRef: 'chapter:vol-01:ch-02',
      draft: {
        facts: [
          { summary: '港口在午夜封锁', confidence: 'accepted' },
          { summary: '调查员怀疑时间走私', source_ref: 'chapter:vol-01:ch-01', confidence: 'weird' },
        ],
      },
    })

    const activeCast = helpers.buildActiveCastDoc({
      existing: {
        schema_version: 2,
        scope_ref: 'chapter:vol-01:ch-01',
        updated_at: 5,
        cast: [
          {
            character_ref: 'char:alice',
            role_in_scope: 'investigator',
            current_state_summary: '旧状态',
            sensitivity_flags: ['spoiler'],
          },
        ],
      },
      scopeRef: 'chapter:vol-01:ch-02',
      draft: {
        cast: [
          {
            character_ref: 'char:alice',
            current_state_summary: '开始怀疑同伴',
            must_keep_voice_signals: ['冷静', '简短'],
          },
        ],
      },
    })

    const checks = [
      createCheck(
        'scope_ref_from_chapter_path_normalizes_manuscripts_prefix',
        helpers.normalizeScopeRefFromChapterPath('manuscripts/vol-01/ch 02.md') === 'chapter:vol-01:ch:02',
      ),
      createCheck(
        'token_budget_prefers_explicit_macro_budget',
        helpers.resolveTokenBudget({ workflowKind: 'book', macroBudget: 'small' }) === 'small'
          && helpers.resolveTokenBudget({ workflowKind: 'micro' }) === 'small'
          && helpers.resolveTokenBudget({ workflowKind: 'chapter' }) === 'medium'
          && helpers.resolveTokenBudget({ workflowKind: 'arc' }) === 'large',
      ),
      createCheck(
        'chapter_card_builder_fills_required_fields_and_preserves_semantics',
        chapterCard.schema_version === 3
          && chapterCard.scope_ref === 'chapter:vol-01:ch-02'
          && chapterCard.scope_locator === 'vol-01/ch-02.md'
          && chapterCard.workflow_kind === 'arc'
          && chapterCard.status === 'active'
          && chapterCard.updated_at === 1700000000123
          && chapterCard.hard_constraints[0] === '保持第一人称不变',
        chapterCard,
      ),
      createCheck(
        'recent_facts_builder_injects_source_ref_and_normalizes_confidence',
        recentFacts.schema_version === helpers.LAYER1_SCHEMA_VERSION
          && recentFacts.updated_at === 1700000000123
          && recentFacts.facts[0]?.source_ref === 'manual:mission-panel:chapter:vol-01:ch-02'
          && recentFacts.facts[0]?.confidence === 'accepted'
          && recentFacts.facts[1]?.source_ref === 'chapter:vol-01:ch-01'
          && recentFacts.facts[1]?.confidence === 'proposed',
        recentFacts,
      ),
      createCheck(
        'active_cast_builder_preserves_existing_optional_fields_per_character',
        activeCast.schema_version === 2
          && activeCast.scope_ref === 'chapter:vol-01:ch-02'
          && activeCast.updated_at === 1700000000123
          && activeCast.cast[0]?.role_in_scope === 'investigator'
          && activeCast.cast[0]?.sensitivity_flags?.[0] === 'spoiler'
          && activeCast.cast[0]?.must_keep_voice_signals?.[1] === '简短',
        activeCast,
      ),
    ]

    const allPass = checks.every((check) => check.pass)
    console.log('[mission-layer1-repair]', JSON.stringify({ all_pass: allPass, checks }))

    if (!allPass) {
      process.exit(1)
    }
  } finally {
    Date.now = originalNow
  }
}

main().catch((error) => {
  console.error('[mission-layer1-repair] failed:', error.message)
  process.exit(1)
})
