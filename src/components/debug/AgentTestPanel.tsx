import { useMemo, useState } from 'react'

import { Button, Input } from '@/magic-ui/components'
import { runAgentTurn, useAgentSessionStore } from '@/lib/agent-test'
import type { AgentMessage } from '@/lib/agent-test/session-store'
import type { ToolTrace } from '@/lib/agent-test/trace'

type AgentTrace = ToolTrace
type AgentTestMessage = AgentMessage

function AgentTestInput(input: {
  value: string
  running: boolean
  turn: number
  lastReply: string
  onChange: (value: string) => void
  onRun: () => void
}) {
  return (
    <div className="p-3 border-b border-border space-y-2">
      <div className="flex gap-2">
        <Input
          value={input.value}
          onChange={(e) => input.onChange(e.target.value)}
          className="flex-1 h-8"
          placeholder={'create chapter "第一章" / read markdown / preview / commit'}
        />
        <Button
          onClick={input.onRun}
          disabled={input.running}
          size="sm"
          className="disabled:opacity-50"
        >
          {input.running ? 'Running...' : 'Run'}
        </Button>
      </div>
      <div className="text-xs text-muted-foreground">turn: {input.turn}</div>
      {input.lastReply ? <div className="text-xs">reply: {input.lastReply}</div> : null}
    </div>
  )
}

function AgentTraceList({ traces }: { traces: AgentTrace[] }) {
  if (traces.length === 0) {
    return <div className="text-xs text-muted-foreground">暂无 trace</div>
  }

  return (
    <div className="space-y-2">
      {traces.map((trace, index) => (
        <div key={`${trace.call_id}-${index}`} className="text-xs border border-border rounded p-2 bg-card">
          <div>
            #{trace.turn} {trace.tool_name} {trace.status} ({trace.duration_ms}ms)
          </div>
          {trace.error_code ? <div className="text-destructive">{trace.fault_domain}:{trace.error_code}</div> : null}
        </div>
      ))}
    </div>
  )
}

function AgentMessageList({ messages }: { messages: AgentTestMessage[] }) {
  return (
    <div className="space-y-2">
      {messages.slice(-10).map((message, index) => (
        <div key={`${message.ts}-${index}`} className="text-xs border border-border rounded p-2 bg-card">
          <span className="">[{message.role}] </span>
          {message.content}
        </div>
      ))}
    </div>
  )
}

export function AgentTestPanel() {
  const [input, setInput] = useState('read markdown')
  const [running, setRunning] = useState(false)
  const [lastReply, setLastReply] = useState('')

  const { session_id, turn, traces, messages } = useAgentSessionStore()
  const recentTraces = useMemo(() => traces.slice(-10).reverse(), [traces])

  const handleRun = async () => {
    const message = input.trim()
    if (!message || running) return

    setRunning(true)
    try {
      const reply = await runAgentTurn(message)
      setLastReply(reply)
    } finally {
      setRunning(false)
    }
  }

  return (
    <div className="w-full bg-background border-l border-border h-full flex flex-col">
      <div className="p-3 border-b border-border flex items-center justify-between">
        <h2 className="text-sm font-medium">Agent Test</h2>
        <div className="text-xs text-muted-foreground">session: {session_id}</div>
      </div>

      <AgentTestInput
        value={input}
        running={running}
        turn={turn}
        lastReply={lastReply}
        onChange={setInput}
        onRun={handleRun}
      />

      <div className="flex-1 min-h-0 overflow-auto p-3 space-y-4">
        <div>
          <div className="text-xs font-medium mb-2">Tool Traces</div>
          <AgentTraceList traces={recentTraces} />
        </div>

        <div>
          <div className="text-xs font-medium mb-2">Messages</div>
          <AgentMessageList messages={messages} />
        </div>
      </div>
    </div>
  )
}
