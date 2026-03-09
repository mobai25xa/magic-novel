let mermaidModule: typeof import('mermaid') | null = null
let mermaidInitialized = false

async function loadMermaidModule() {
  if (!mermaidModule) {
    mermaidModule = await import('mermaid')
  }
  return mermaidModule.default
}

function initializeMermaidTheme(mermaid: Awaited<ReturnType<typeof loadMermaidModule>>) {
  if (mermaidInitialized) {
    return
  }

  const isDark = document.documentElement.classList.contains('dark')
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'strict',
    flowchart: { useMaxWidth: true },
  })
  mermaidInitialized = true
}

export async function getMermaid() {
  const mermaid = await loadMermaidModule()
  initializeMermaidTheme(mermaid)
  return mermaid
}

export function resetMermaidTheme() {
  mermaidInitialized = false
}
