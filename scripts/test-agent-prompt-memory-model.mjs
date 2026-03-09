import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

async function main() {
  const root = resolve(import.meta.dirname, '..')

  const files = {
    promptCore: await readFile(resolve(root, 'src/agent/prompt/core.ts'), 'utf-8'),
    promptInjections: await readFile(resolve(root, 'src/agent/prompt/injections.ts'), 'utf-8'),
    promptProviderPatches: await readFile(resolve(root, 'src/agent/prompt/provider-patches.ts'), 'utf-8'),
    promptToolDescriptions: await readFile(resolve(root, 'src/agent/prompt/tool-descriptions.ts'), 'utf-8'),
    memoryContextBuilder: await readFile(resolve(root, 'src/agent/memory/context-builder.ts'), 'utf-8'),
    memoryRetrieval: await readFile(resolve(root, 'src/agent/memory/retrieval.ts'), 'utf-8'),
    memoryCompactor: await readFile(resolve(root, 'src/agent/memory/compactor.ts'), 'utf-8'),
    memorySummaries: await readFile(resolve(root, 'src/agent/memory/summaries.ts'), 'utf-8'),
    modelRouter: await readFile(resolve(root, 'src/agent/model/router.ts'), 'utf-8'),
    modelProviders: await readFile(resolve(root, 'src/agent/model/providers.ts'), 'utf-8'),
    modelCapabilities: await readFile(resolve(root, 'src/agent/model/capabilities.ts'), 'utf-8'),
    modelFallback: await readFile(resolve(root, 'src/agent/model/fallback.ts'), 'utf-8'),
    runtimeTurnEngine: await readFile(resolve(root, 'src/agent/runtime/turn-engine.ts'), 'utf-8'),
    runtimeLoopSupport: await readFile(resolve(root, 'src/agent/runtime/loop-support.ts'), 'utf-8'),
  }

  const checks = [
    {
      name: 'prompt_layering_order',
      pass:
        files.memoryContextBuilder.includes('return [split.baseSystem, injected, toolDescription, ...split.others]') &&
        files.memoryContextBuilder.includes('buildToolDescriptionPrompt(\'default\')'),
    },
    {
      name: 'prompt_injection_contract',
      pass:
        files.promptInjections.includes('buildPromptInjectionFields') &&
        files.promptInjections.includes('formatPromptInjection') &&
        files.promptInjections.includes('active_chapter'),
    },
    {
      name: 'provider_patch_minimal_scope',
      pass:
        files.promptProviderPatches.includes('OPENAI_COMPATIBLE_PROMPT_PATCH') &&
        files.promptProviderPatches.includes('provider === \'openai-compatible\'') &&
        files.promptProviderPatches.includes('return null'),
    },
    {
      name: 'tool_description_prompt',
      pass:
        files.promptToolDescriptions.includes('buildToolDescriptionPrompt') &&
        files.promptToolDescriptions.includes('required:') &&
        files.promptToolDescriptions.includes('可用工具：'),
    },
    {
      name: 'memory_compaction_structure',
      pass:
        files.memorySummaries.includes('summaryText') &&
        files.memorySummaries.includes('anchors') &&
        files.memoryCompactor.includes('anchors:') &&
        files.memoryCompactor.includes('compactConversationSafe'),
    },
    {
      name: 'retrieval_hint_stability',
      pass:
        files.memoryRetrieval.includes('type RetrievalHint') &&
        files.memoryRetrieval.includes('formatRetrievalHints') &&
        files.memoryContextBuilder.includes('formatRetrievalHints(buildRetrievalHints(snapshot))'),
    },
    {
      name: 'model_router_and_fallback',
      pass:
        files.modelRouter.includes('routeModelWithInput') &&
        files.modelRouter.includes('resolveEnabledModels') &&
        files.modelProviders.includes('getModelProviderByName') &&
        files.modelFallback.includes('withModelFallbackMeta') &&
        files.modelFallback.includes('provider_unavailable'),
    },
    {
      name: 'capabilities_and_tool_downgrade',
      pass:
        files.modelCapabilities.includes('CAPABILITIES_BY_PROVIDER') &&
        files.runtimeTurnEngine.includes('toolsEnabled = capabilities.supportsTools') &&
        files.runtimeTurnEngine.includes('tools: toolsEnabled ? input.tools : undefined'),
    },
    {
      name: 'compaction_fallback_path',
      pass:
        files.runtimeLoopSupport.includes('compactConversationSafe') &&
        files.runtimeLoopSupport.includes('E_AGENT_CONTEXT_LIMIT'),
    },
  ]

  const allPass = checks.every((check) => check.pass)
  console.log('[test-agent-prompt-memory-model]', JSON.stringify({ all_pass: allPass, checks }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[test-agent-prompt-memory-model] failed:', error.message)
  process.exit(1)
})
