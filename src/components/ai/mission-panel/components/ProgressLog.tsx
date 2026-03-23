export function ProgressLog({ entries }: { entries: Array<{ ts: number; message: string }> }) {
  if (entries.length === 0) return null
  return (
    <div className="mt-2 max-h-28 overflow-y-auto space-y-0.5 text-xs font-mono">
      {entries
        .slice()
        .reverse()
        .map((e, i) => (
          <div key={i}>
            <span className="opacity-50">{new Date(e.ts).toLocaleTimeString()} </span>
            {e.message}
          </div>
        ))}
    </div>
  )
}

