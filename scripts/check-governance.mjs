import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { dirname, extname, relative, resolve } from 'node:path'

const projectRoot = resolve(import.meta.dirname, '..')
const workspaceRoot = resolve(projectRoot, '..')
const artifactsDir = resolve(workspaceRoot, 'docs', 'magic_plan', 'plan_reconstruction', '_artifacts')
const reportPath = resolve(artifactsDir, 'phase6-governance-report.json')
const baselinePath = resolve(projectRoot, 'scripts', 'governance-baseline.json')

const strict = process.argv.includes('--strict')
const updateBaseline = process.argv.includes('--update-baseline')

const FILE_LINE_LIMITS = {
  '.ts': 300,
  '.tsx': 300,
  '.rs': 400,
}
const MAX_FUNCTION_LINES = 80
const MAX_FILES_PER_DIR = 15
const SOURCE_ROOTS = [resolve(projectRoot, 'src'), resolve(projectRoot, 'src-tauri', 'src')]
const UI_ROOTS = [resolve(projectRoot, 'src', 'components'), resolve(projectRoot, 'src', 'pages')]
const COMMANDS_ROOT = resolve(projectRoot, 'src-tauri', 'src', 'commands')

const REQUIRED_DOCS = [
  resolve(workspaceRoot, 'docs', 'magic_nover', 'cataloga_tall.md'),
  resolve(workspaceRoot, 'docs', 'magic_nover', 'magic_tool', 'tool_contract.md'),
  resolve(workspaceRoot, 'docs', 'magic_nover', 'magic_tool', 'tool_description.md'),
  resolve(workspaceRoot, 'docs', 'magic_plan', 'plan_reconstruction', '00-overview-and-governance.md'),
  resolve(workspaceRoot, 'docs', 'magic_plan', 'plan_reconstruction', '07-phase6-docs-governance-and-dod.md'),
]

const BANNED_UI_IMPORT_PREFIXES = [
  '@/lib/tauri-commands',
  '@/platform/tauri/clients',
  '@/lib/tool-gateway',
  '@/src-tauri',
]

const COMMAND_BUSINESS_SIGNALS = [
  { code: 'std_fs', pattern: /std::fs::/g },
  { code: 'pathbuf', pattern: /PathBuf::from\(/g },
  { code: 'read_json', pattern: /\bread_json\(/g },
  { code: 'write_json', pattern: /\bwrite_json\(/g },
  { code: 'atomic_write_json', pattern: /\batomic_write_json\(/g },
  { code: 'serde_json_parse', pattern: /serde_json::from_str/g },
]

const EMPTY_BASELINE = {
  file_length: [],
  function_length: [],
  directory_file_count: [],
  ui_layer_dependency: [],
  command_business_logic: [],
}

async function main() {
  const findings = await collectFindings()

  const baseline = await loadBaseline()
  const baselineSnapshot = buildBaselineSnapshot(findings)
  const delta = diffAgainstBaseline(findings, baseline, strict)

  if (updateBaseline) {
    await writeJson(baselinePath, {
      version: 1,
      generated_at: new Date().toISOString(),
      rules: {
        file_line_limits: FILE_LINE_LIMITS,
        max_function_lines: MAX_FUNCTION_LINES,
        max_files_per_dir: MAX_FILES_PER_DIR,
      },
      violations: baselineSnapshot,
    })
  }

  const report = {
    generated_at: new Date().toISOString(),
    strict,
    updateBaseline,
    config: {
      file_line_limits: FILE_LINE_LIMITS,
      max_function_lines: MAX_FUNCTION_LINES,
      max_files_per_dir: MAX_FILES_PER_DIR,
    },
    docs: {
      required: REQUIRED_DOCS.map(toWorkspacePath),
      missing: findings.required_docs_missing,
    },
    findings,
    baseline: {
      path: toProjectPath(baselinePath),
      loaded: baseline.loaded,
      new_violations: delta.newViolations,
      resolved_violations: delta.resolvedViolations,
    },
    summary: {
      missing_required_docs: findings.required_docs_missing.length,
      total_current_violations: countCurrentViolations(findings),
      baseline_recorded_violations: countBaselineViolations(baselineSnapshot),
      new_violations: countBaselineViolations(delta.newViolations),
      resolved_violations: countBaselineViolations(delta.resolvedViolations),
      pass: delta.pass,
    },
  }

  await mkdir(artifactsDir, { recursive: true })
  await writeJson(reportPath, report)

  console.log('[governance]', JSON.stringify({
    pass: report.summary.pass,
    strict,
    updateBaseline,
    missing_required_docs: report.summary.missing_required_docs,
    total_current_violations: report.summary.total_current_violations,
    new_violations: report.summary.new_violations,
    report: toWorkspacePath(reportPath),
    baseline: toProjectPath(baselinePath),
  }))

  if (!report.summary.pass) {
    process.exit(1)
  }
}

async function collectFindings() {
  const allCodeFiles = []
  for (const root of SOURCE_ROOTS) {
    const files = await walkFiles(root)
    for (const file of files) {
      const ext = extname(file)
      if (!Object.hasOwn(FILE_LINE_LIMITS, ext)) continue
      allCodeFiles.push(file)
    }
  }

  const required_docs_missing = []
  for (const path of REQUIRED_DOCS) {
    if (!(await exists(path))) {
      required_docs_missing.push(toWorkspacePath(path))
    }
  }

  const file_length = []
  const function_length = []

  for (const file of allCodeFiles) {
    const ext = extname(file)
    const maxLines = FILE_LINE_LIMITS[ext]
    const text = await readFile(file, 'utf-8')
    const lines = splitLines(text)

    if (lines.length > maxLines) {
      file_length.push({
        path: toProjectPath(file),
        lines: lines.length,
        max: maxLines,
      })
    }

    const fnSpans = extractFunctionSpans(lines, ext)
    for (const fn of fnSpans) {
      if (fn.lines > MAX_FUNCTION_LINES) {
        function_length.push({
          path: toProjectPath(file),
          name: fn.name,
          start_line: fn.startLine,
          end_line: fn.endLine,
          lines: fn.lines,
          max: MAX_FUNCTION_LINES,
        })
      }
    }
  }

  const directory_file_count = collectDirectoryFileCountViolations(allCodeFiles)
  const ui_layer_dependency = await collectUiLayerDependencyViolations()
  const command_business_logic = await collectCommandBusinessLogicViolations()

  return {
    required_docs_missing,
    file_length,
    function_length,
    directory_file_count,
    ui_layer_dependency,
    command_business_logic,
  }
}

function extractFunctionSpans(lines, ext) {
  const spans = []
  const patterns = ext === '.rs'
    ? [/^(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\(/]
    : [
      /^(?:export\s+)?(?:async\s+)?function\s+([A-Za-z0-9_$]+)\s*\(/,
      /^(?:export\s+)?const\s+([A-Za-z0-9_$]+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>/,
      /^(?:const|let|var)\s+([A-Za-z0-9_$]+)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>/,
    ]

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i].trim()
    if (!line || line.startsWith('//')) continue

    const match = patterns
      .map((pattern) => line.match(pattern))
      .find(Boolean)

    if (!match) continue

    const name = match[1] || 'anonymous'
    const block = findFunctionBlock(lines, i)
    if (!block) continue

    spans.push({
      name,
      startLine: i + 1,
      endLine: block.end + 1,
      lines: block.end - i + 1,
    })
  }

  return spans
}

function findFunctionBlock(lines, start) {
  let started = false
  let depth = 0
  for (let i = start; i < lines.length; i += 1) {
    const line = lines[i]
    const opens = countChar(line, '{')
    const closes = countChar(line, '}')

    if (!started && opens > 0) {
      started = true
    }

    if (!started) continue

    depth += opens
    depth -= closes

    if (started && depth <= 0) {
      return { end: i }
    }
  }

  return null
}

function collectDirectoryFileCountViolations(files) {
  const dirCounts = new Map()

  for (const file of files) {
    const dir = dirname(toProjectPath(file)).replace(/\\/g, '/')
    dirCounts.set(dir, (dirCounts.get(dir) || 0) + 1)
  }

  const violations = []
  for (const [dir, count] of dirCounts.entries()) {
    if (count > MAX_FILES_PER_DIR) {
      violations.push({
        dir,
        count,
        max: MAX_FILES_PER_DIR,
      })
    }
  }

  return violations.sort((a, b) => b.count - a.count)
}

async function collectUiLayerDependencyViolations() {
  const violations = []

  for (const root of UI_ROOTS) {
    const files = await walkFiles(root)
    for (const file of files) {
      const ext = extname(file)
      if (ext !== '.ts' && ext !== '.tsx') continue

      const content = await readFile(file, 'utf-8')
      const imports = extractImports(content)
      for (const item of imports) {
        if (!BANNED_UI_IMPORT_PREFIXES.some((prefix) => item.startsWith(prefix))) continue
        violations.push({
          path: toProjectPath(file),
          import_path: item,
        })
      }
    }
  }

  return violations
}

async function collectCommandBusinessLogicViolations() {
  const violations = []
  const files = await walkFiles(COMMANDS_ROOT)

  for (const file of files) {
    if (extname(file) !== '.rs') continue
    if (file.endsWith('mod.rs')) continue

    const content = await readFile(file, 'utf-8')
    const hits = []

    for (const signal of COMMAND_BUSINESS_SIGNALS) {
      const count = countMatches(content, signal.pattern)
      if (count > 0) {
        hits.push({ code: signal.code, count })
      }
    }

    const total = hits.reduce((sum, h) => sum + h.count, 0)
    if (total >= 3) {
      violations.push({
        path: toProjectPath(file),
        signal_total: total,
        signals: hits,
      })
    }
  }

  return violations
}

function buildBaselineSnapshot(findings) {
  return {
    file_length: findings.file_length.map((v) => `${v.path}>${v.max}`),
    function_length: findings.function_length.map((v) => `${v.path}::${v.name}`),
    directory_file_count: findings.directory_file_count.map((v) => v.dir),
    ui_layer_dependency: findings.ui_layer_dependency.map((v) => `${v.path} -> ${v.import_path}`),
    command_business_logic: findings.command_business_logic.map((v) => v.path),
  }
}

function diffAgainstBaseline(findings, baseline, strictMode) {
  const current = buildBaselineSnapshot(findings)
  const base = baseline.violations

  const newViolations = {}
  const resolvedViolations = {}

  for (const key of Object.keys(EMPTY_BASELINE)) {
    const currentSet = new Set(current[key] || [])
    const baselineSet = new Set(base[key] || [])

    newViolations[key] = Array.from(currentSet).filter((item) => !baselineSet.has(item))
    resolvedViolations[key] = Array.from(baselineSet).filter((item) => !currentSet.has(item))
  }

  const hasMissingDocs = findings.required_docs_missing.length > 0
  const hasAnyCurrentViolation = countBaselineViolations(current) > 0
  const hasAnyNewViolation = countBaselineViolations(newViolations) > 0

  const pass = strictMode
    ? !hasMissingDocs && !hasAnyCurrentViolation
    : !hasMissingDocs && !hasAnyNewViolation

  return {
    pass,
    newViolations,
    resolvedViolations,
  }
}

async function loadBaseline() {
  if (!(await exists(baselinePath))) {
    return {
      loaded: false,
      violations: { ...EMPTY_BASELINE },
    }
  }

  try {
    const raw = await readFile(baselinePath, 'utf-8')
    const parsed = JSON.parse(raw)
    const violations = {
      ...EMPTY_BASELINE,
      ...(parsed?.violations || {}),
    }
    return { loaded: true, violations }
  } catch {
    return {
      loaded: false,
      violations: { ...EMPTY_BASELINE },
    }
  }
}

function extractImports(source) {
  const imports = []
  const regex = /from\s+['"]([^'"]+)['"]/g
  let match = regex.exec(source)
  while (match) {
    imports.push(match[1])
    match = regex.exec(source)
  }
  return imports
}

async function walkFiles(root) {
  if (!(await exists(root))) return []
  const out = []

  async function walk(dir) {
    const entries = await readdir(dir, { withFileTypes: true })
    for (const entry of entries) {
      const full = resolve(dir, entry.name)
      const rel = full.replace(projectRoot, '')
      if (isIgnoredPath(rel)) continue

      if (entry.isDirectory()) {
        await walk(full)
        continue
      }

      if (entry.isFile()) {
        out.push(full)
      }
    }
  }

  await walk(root)
  return out
}

function isIgnoredPath(path) {
  return [
    '\\node_modules\\',
    '\\dist\\',
    '\\target\\',
    '\\.git\\',
    '\\coverage\\',
    '\\.factory\\',
  ].some((token) => path.includes(token))
}

function splitLines(text) {
  if (!text) return []
  return text.replace(/\r\n/g, '\n').split('\n')
}

function countChar(input, char) {
  let count = 0
  for (const c of input) {
    if (c === char) count += 1
  }
  return count
}

function countMatches(input, regex) {
  const matches = input.match(regex)
  return matches ? matches.length : 0
}

function toProjectPath(absolutePath) {
  return relative(projectRoot, absolutePath).replace(/\\/g, '/')
}

function toWorkspacePath(absolutePath) {
  return relative(workspaceRoot, absolutePath).replace(/\\/g, '/')
}

async function exists(path) {
  try {
    await readdir(path)
    return true
  } catch {
    try {
      await readFile(path)
      return true
    } catch {
      return false
    }
  }
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true })
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, 'utf-8')
}

function countCurrentViolations(findings) {
  return (
    findings.file_length.length +
    findings.function_length.length +
    findings.directory_file_count.length +
    findings.ui_layer_dependency.length +
    findings.command_business_logic.length
  )
}

function countBaselineViolations(snapshot) {
  return Object.values(snapshot)
    .filter((v) => Array.isArray(v))
    .reduce((sum, arr) => sum + arr.length, 0)
}

main().catch((error) => {
  console.error('[governance] failed:', error.message)
  process.exit(1)
})
