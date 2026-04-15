import {
  inspirationTurnCancelClient,
  inspirationTurnStartClient,
} from '@/platform/tauri/clients/inspiration-engine-client'
import {
  inspirationSessionCreateClient,
  inspirationSessionDeleteClient,
  inspirationSessionListClient,
  inspirationSessionLoadClient,
  inspirationSessionSaveStateClient,
  inspirationSessionUpdateMetaClient,
  type InspirationSessionMeta,
} from '@/platform/tauri/clients/inspiration-session-client'
import {
  inspirationGenerateMetadataVariantsClient,
  type InspirationMetadataVariantCandidate,
} from '@/platform/tauri/clients/inspiration-variants-client'

export * from './types'

export type { InspirationMetadataVariantCandidate, InspirationSessionMeta }

export {
  inspirationGenerateMetadataVariantsClient,
  inspirationSessionCreateClient,
  inspirationSessionDeleteClient,
  inspirationSessionListClient,
  inspirationSessionLoadClient,
  inspirationSessionSaveStateClient,
  inspirationSessionUpdateMetaClient,
  inspirationTurnCancelClient,
  inspirationTurnStartClient,
}
