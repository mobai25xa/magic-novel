/**
 * @author Gamma
 * @description 生成不同规模的测试文档
 * 使用方法：在 Console 中粘贴运行
 */

// 生成指定字数的 TipTap JSON 文档
function generateTestDoc(wordCount: number): object {
  const paragraphs = []
  const wordsPerParagraph = 100
  const paragraphCount = Math.ceil(wordCount / wordsPerParagraph)

  for (let i = 0; i < paragraphCount; i++) {
    const text = generateChineseText(wordsPerParagraph)
    paragraphs.push({
      type: 'paragraph',
      attrs: { id: crypto.randomUUID() },
      content: [{ type: 'text', text }],
    })
  }

  return {
    type: 'doc',
    content: paragraphs,
  }
}

function generateChineseText(count: number): string {
  const sample = '这是一段用于性能测试的中文文本内容。在这个美丽的故事中，主人公经历了许多精彩的冒险。'
  let result = ''
  while (result.length < count) {
    result += sample
  }
  return result.slice(0, count)
}

// 使用方式：
// const doc = generateTestDoc(50000)  // 5万字文档
// editor.api.getContent('json') 保存当前内容
// 然后 editor 实例的 setContent(doc) 加载测试文档

export { generateTestDoc, generateChineseText }
