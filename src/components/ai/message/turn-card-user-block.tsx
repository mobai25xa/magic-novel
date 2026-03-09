import { useEffect, useState } from 'react'

import { MessageTimestamp } from './message-timestamp'

type TurnCardUserBlockProps = {
  userText: string
  timestamp?: number
}

export function TurnCardUserBlock(input: TurnCardUserBlockProps) {
  const [isFirst, setIsFirst] = useState(true)

  useEffect(() => {
    queueMicrotask(() => {
      setIsFirst(false)
    })
  }, [])

  if (!input.userText.trim()) {
    return null
  }

  return (
    <div className={`flex flex-col items-end gap-1 ${isFirst ? 'ai-animate-fly-in' : ''}`}>
      <div className="editor-shell-ai-user-bubble max-w-[84%] rounded-[14px_14px_4px_14px] border px-3.5 py-2.5 text-[13.5px] whitespace-pre-wrap break-words leading-[1.6] shadow-[0_1px_2px_rgba(0,0,0,0.02)]">
        {input.userText}
      </div>
      {input.timestamp ? (
        <MessageTimestamp timestamp={input.timestamp} className="editor-shell-ai-user-time" />
      ) : null}
    </div>
  )
}
