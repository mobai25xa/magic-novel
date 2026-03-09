import type { AgentSessionMeta } from '@/lib/agent-chat/session/session-types'
import type {
  AgentSessionAppendEventsInput,
  AgentSessionCreateInput,
  AgentSessionCreateOutput,
  AgentSessionDeleteInput,
  AgentSessionHydrateInput,
  AgentSessionHydrateOutput,
  AgentSessionListInput,
  AgentSessionLoadInput,
  AgentSessionLoadOutput,
  AgentSessionRecoverInput,
  AgentSessionRecoverOutput,
  AgentSessionUpdateMetaInput,
} from '@/platform/tauri/clients/agent-session-client'
import {
  agentSessionAppendEventsClient,
  agentSessionCreateClient,
  agentSessionDeleteClient,
  agentSessionHydrateClient,
  agentSessionListClient,
  agentSessionLoadClient,
  agentSessionRecoverClient,
  agentSessionUpdateMetaClient,
} from '@/platform/tauri/clients/agent-session-client'

export async function agentSessionCreate(input: AgentSessionCreateInput): Promise<AgentSessionCreateOutput> {
  return agentSessionCreateClient(input)
}

export async function agentSessionAppendEvents(input: AgentSessionAppendEventsInput): Promise<void> {
  return agentSessionAppendEventsClient(input)
}

export async function agentSessionLoad(input: AgentSessionLoadInput): Promise<AgentSessionLoadOutput> {
  return agentSessionLoadClient(input)
}

export async function agentSessionHydrate(input: AgentSessionHydrateInput): Promise<AgentSessionHydrateOutput> {
  return agentSessionHydrateClient(input)
}

export async function agentSessionList(input: AgentSessionListInput): Promise<AgentSessionMeta[]> {
  return agentSessionListClient(input)
}

export async function agentSessionUpdateMeta(input: AgentSessionUpdateMetaInput): Promise<void> {
  return agentSessionUpdateMetaClient(input)
}

export async function agentSessionRecover(input: AgentSessionRecoverInput): Promise<AgentSessionRecoverOutput> {
  return agentSessionRecoverClient(input)
}

export async function agentSessionDelete(input: AgentSessionDeleteInput): Promise<void> {
  return agentSessionDeleteClient(input)
}
