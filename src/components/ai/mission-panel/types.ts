import type {
  missionContextpackGetLatestFeature,
  missionGetStatusFeature,
  missionKnowledgeGetLatestFeature,
  missionLayer1GetFeature,
  missionMacroGetStateFeature,
  missionReviewGetLatestFeature,
  missionReviewGetPendingDecisionFeature,
  missionReviewListFeature,
} from '@/features/agent-chat'

export type MissionPanelProps = {
  projectPath: string
  missionId: string
  /** Optional: called when user requests to close the panel */
  onClose?: () => void
}

export type MissionStatusPayload = Awaited<ReturnType<typeof missionGetStatusFeature>>
export type Layer1SnapshotPayload = Awaited<ReturnType<typeof missionLayer1GetFeature>>
export type ContextPackPayload = Awaited<ReturnType<typeof missionContextpackGetLatestFeature>>
export type ReviewReportPayload = Awaited<ReturnType<typeof missionReviewGetLatestFeature>>
export type ReviewHistoryPayload = Awaited<ReturnType<typeof missionReviewListFeature>>
export type ReviewDecisionPayload = Awaited<ReturnType<typeof missionReviewGetPendingDecisionFeature>>
export type KnowledgeLatestPayload = Awaited<ReturnType<typeof missionKnowledgeGetLatestFeature>>
export type MacroStatePayload = Awaited<ReturnType<typeof missionMacroGetStateFeature>>

