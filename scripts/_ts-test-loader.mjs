import { existsSync, readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, extname, resolve } from 'node:path'
import * as ts from 'typescript'

const nodeRequire = createRequire(import.meta.url)

function resolveExistingFile(basePath) {
  const direct = extname(basePath)
  const candidates = direct
    ? [basePath]
    : [
        `${basePath}.ts`,
        `${basePath}.tsx`,
        `${basePath}.js`,
        `${basePath}.mjs`,
        resolve(basePath, 'index.ts'),
        resolve(basePath, 'index.tsx'),
        resolve(basePath, 'index.js'),
        resolve(basePath, 'index.mjs'),
      ]

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate
    }
  }

  return null
}

export function createTsModuleLoader(rootDir) {
  const cache = new Map()

  function resolveImport(fromFile, specifier) {
    if (specifier.startsWith('node:')) {
      return { kind: 'node', value: specifier }
    }

    if (specifier.startsWith('@/')) {
      const resolved = resolveExistingFile(resolve(rootDir, 'src', specifier.slice(2)))
      if (!resolved) {
        throw new Error(`Unable to resolve alias import: ${specifier} from ${fromFile}`)
      }

      return { kind: 'file', value: resolved }
    }

    if (specifier.startsWith('.')) {
      const resolved = resolveExistingFile(resolve(dirname(fromFile), specifier))
      if (!resolved) {
        throw new Error(`Unable to resolve relative import: ${specifier} from ${fromFile}`)
      }

      return { kind: 'file', value: resolved }
    }

    return { kind: 'node', value: specifier }
  }

  function loadModule(filePath) {
    const normalizedPath = resolve(filePath)
    if (cache.has(normalizedPath)) {
      return cache.get(normalizedPath)
    }

    const source = readFileSync(normalizedPath, 'utf-8')
    const transpiled = ts.transpileModule(source, {
      fileName: normalizedPath,
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.Node10,
        jsx: ts.JsxEmit.ReactJSX,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true,
      },
    })

    const module = { exports: {} }
    cache.set(normalizedPath, module.exports)

    const localRequire = (specifier) => {
      const resolved = resolveImport(normalizedPath, specifier)
      if (resolved.kind === 'node') {
        return nodeRequire(resolved.value)
      }

      return loadModule(resolved.value)
    }

    const runner = new Function('require', 'module', 'exports', '__filename', '__dirname', transpiled.outputText)
    runner(localRequire, module, module.exports, normalizedPath, dirname(normalizedPath))

    cache.set(normalizedPath, module.exports)
    return module.exports
  }

  return {
    loadModule,
  }
}
