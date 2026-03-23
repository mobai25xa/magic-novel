import type {
  CreateProjectHandoffDraft,
  InspirationConsensusState,
  MetadataVariant,
} from '@/features/inspiration/types'

import { invokeTauri } from './core'

export interface InspirationGenerateMetadataVariantsInput {
  consensus: InspirationConsensusState
}

export interface InspirationMetadataVariantCandidate {
  variant: MetadataVariant
  create_handoff: CreateProjectHandoffDraft
}

export interface InspirationGenerateMetadataVariantsOutput {
  schema_version: number
  shared_story_core: string
  variants: InspirationMetadataVariantCandidate[]
}

export async function inspirationGenerateMetadataVariantsClient(
  input: InspirationGenerateMetadataVariantsInput,
): Promise<InspirationGenerateMetadataVariantsOutput> {
  return invokeTauri<InspirationGenerateMetadataVariantsOutput>(
    'inspiration_generate_metadata_variants',
    { input },
  )
}
