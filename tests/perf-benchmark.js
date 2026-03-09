/**
 * 性能基准测试脚本
 * 在应用的 DevTools Console 中粘贴运行
 */
async function runBenchmarks() {
  const results = {}

  // T-9.2 打开速度
  const sizes = [1000, 5000, 20000, 50000]
  for (const size of sizes) {
    const doc = generateTestDoc(size)
    const start = performance.now()
    window.__tiptapEditor?.commands.setContent(doc)
    const end = performance.now()
    results[`open_${size}`] = `${(end - start).toFixed(1)}ms`
    await sleep(500)
  }

  // T-9.3 保存速度
  const saveStart = performance.now()
  await window.__manualSave?.()
  const saveEnd = performance.now()
  results['save'] = `${(saveEnd - saveStart).toFixed(1)}ms`

  // T-9.5 内存
  if (performance.memory) {
    results['memory_mb'] = `${(performance.memory.usedJSHeapSize / 1024 / 1024).toFixed(1)}MB`
  }

  console.table(results)
  return results
}

function sleep(ms) { return new Promise(r => setTimeout(r, ms)) }

function generateTestDoc(wordCount) {
  const paragraphs = []
  const sample = '这是一段用于性能测试的中文文本内容。在这个美丽的故事中主人公经历了许多精彩的冒险和奇遇。'
  const wordsPerParagraph = 100
  const count = Math.ceil(wordCount / wordsPerParagraph)
  for (let i = 0; i < count; i++) {
    let text = ''
    while (text.length < wordsPerParagraph) text += sample
    text = text.slice(0, wordsPerParagraph)
    paragraphs.push({
      type: 'paragraph',
      attrs: { id: crypto.randomUUID() },
      content: [{ type: 'text', text }],
    })
  }
  return { type: 'doc', content: paragraphs }
}

// 运行
runBenchmarks()
