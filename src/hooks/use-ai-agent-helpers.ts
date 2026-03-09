export function extractPlainTextFromDoc(doc: unknown): string {
  const fragments: string[] = []

  const walk = (node: unknown) => {
    if (!node || typeof node !== 'object') return

    const record = node as Record<string, unknown>
    if (record.type === 'text' && typeof record.text === 'string') {
      fragments.push(record.text)
      return
    }

    if (Array.isArray(record.content)) {
      record.content.forEach(walk)
    }
  }

  walk(doc)
  return fragments.join('')
}

export async function calculateDocHash(doc: unknown): Promise<string> {
  const content = JSON.stringify(doc)
  const encoded = new TextEncoder().encode(content)
  const hashBuffer = await crypto.subtle.digest('SHA-256', encoded)
  const hashArray = Array.from(new Uint8Array(hashBuffer))
  return hashArray.map((byte) => byte.toString(16).padStart(2, '0')).join('').slice(0, 16)
}
