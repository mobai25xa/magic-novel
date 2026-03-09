import type { FaultDomain } from '@/lib/tool-gateway/types'

export interface ToolTrace {
  turn: number
  call_id: string
  tool_name: string
  status: 'ok' | 'error'
  fault_domain?: FaultDomain
  error_code?: string
  duration_ms: number
}

export function traceSuccess(input: {
  turn: number
  call_id: string
  tool_name: string
  duration_ms: number
}): ToolTrace {
  return {
    turn: input.turn,
    call_id: input.call_id,
    tool_name: input.tool_name,
    status: 'ok',
    duration_ms: input.duration_ms,
  }
}

export function traceError(input: {
  turn: number
  call_id: string
  tool_name: string
  duration_ms: number
  fault_domain?: FaultDomain
  error_code?: string
}): ToolTrace {
  return {
    turn: input.turn,
    call_id: input.call_id,
    tool_name: input.tool_name,
    status: 'error',
    duration_ms: input.duration_ms,
    fault_domain: input.fault_domain,
    error_code: input.error_code,
  }
}
