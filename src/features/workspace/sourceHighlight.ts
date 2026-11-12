import type { HighlighterCore } from 'shiki/types'

// 显式列出动态导入，构建只包含这些语言，运行时不请求远程资源。
const languages = {
  csharp: () => import('@shikijs/langs/csharp'),
  lua: () => import('@shikijs/langs/lua'),
  json: () => import('@shikijs/langs/json'),
  xml: () => import('@shikijs/langs/xml'),
  javascript: () => import('@shikijs/langs/javascript'),
  typescript: () => import('@shikijs/langs/typescript'),
  vue: () => import('@shikijs/langs/vue'),
  css: () => import('@shikijs/langs/css'),
  rust: () => import('@shikijs/langs/rust'),
  yaml: () => import('@shikijs/langs/yaml'),
  markdown: () => import('@shikijs/langs/markdown'),
  python: () => import('@shikijs/langs/python'),
  shellscript: () => import('@shikijs/langs/shellscript'),
  c: () => import('@shikijs/langs/c'),
  cpp: () => import('@shikijs/langs/cpp'),
} as const
type SourceLanguage = keyof typeof languages
const extensions: Record<string, SourceLanguage> = {
  cs: 'csharp',
  lua: 'lua',
  json: 'json',
  xml: 'xml',
  csproj: 'xml',
  props: 'xml',
  targets: 'xml',
  xaml: 'xml',
  svg: 'xml',
  html: 'xml',
  htm: 'xml',
  js: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  jsx: 'javascript',
  ts: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  tsx: 'typescript',
  vue: 'vue',
  css: 'css',
  rs: 'rust',
  yaml: 'yaml',
  yml: 'yaml',
  md: 'markdown',
  py: 'python',
  sh: 'shellscript',
  bash: 'shellscript',
  c: 'c',
  h: 'c',
  cpp: 'cpp',
  hpp: 'cpp',
  cc: 'cpp',
}
let highlighterPromise: Promise<HighlighterCore> | undefined
const loadingLanguages = new Map<SourceLanguage, Promise<void>>()

/** 首次预览代码时初始化，切换文件或放大预览时复用同一个实例。 */
function getHighlighter(): Promise<HighlighterCore> {
  highlighterPromise ??= Promise.all([
    import('shiki/core'),
    import('shiki/engine/oniguruma'),
    import('@shikijs/themes/github-light'),
    import('@shikijs/themes/github-dark'),
  ]).then(([core, engine, light, dark]) => core.createHighlighterCore({
    themes: [light.default, dark.default],
    langs: [],
    engine: engine.createOnigurumaEngine(import('shiki/wasm')),
  })).catch((cause) => {
    highlighterPromise = undefined
    throw cause
  })
  return highlighterPromise
}

/** 根据文件后缀生成已转义的高亮 HTML；未知类型返回空值，保留原文显示。 */
export async function highlightSource(code: string, path: string): Promise<string | undefined> {
  const extension = path.split('/').pop()?.split('.').pop()?.toLowerCase() ?? ''
  const language = extensions[extension]
  if (!language) {
    return undefined
  }
  const highlighter = await getHighlighter()
  if (!loadingLanguages.has(language)) {
    loadingLanguages.set(language, highlighter.loadLanguage(languages[language]).catch((cause) => {
      loadingLanguages.delete(language)
      throw cause
    }))
  }
  await loadingLanguages.get(language)
  return highlighter.codeToHtml(code, { lang: language, themes: { light: 'github-light', dark: 'github-dark' }, defaultColor: false })
}
