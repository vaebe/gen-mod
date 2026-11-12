import type { SourceEntry } from '@/contracts/source'
import { shallowRef } from 'vue'
import { workspaceService } from './workspaceService'

export interface SourceReference extends SourceEntry { invalid?: boolean }
export const sourceReferences = shallowRef<Record<string, SourceReference[]>>({})
export const sourceLeaveGuards = new Map<string, () => Promise<boolean>>()

export function addSourceReferences(modId: string, entries: SourceEntry[]): void {
  const current = sourceReferences.value[modId] ?? []
  sourceReferences.value = { ...sourceReferences.value, [modId]: [...current, ...entries.filter(entry => entry.path && entry.kind !== 'blocked' && !current.some(item => item.path === entry.path))] }
}

export function removeSourceReference(modId: string, path: string): void {
  sourceReferences.value = { ...sourceReferences.value, [modId]: (sourceReferences.value[modId] ?? []).filter(item => item.path !== path) }
}

export function remapSourceReferences(modId: string, source: string, target?: string): void {
  sourceReferences.value = { ...sourceReferences.value, [modId]: (sourceReferences.value[modId] ?? []).map((entry) => {
    if (entry.path !== source && !entry.path.startsWith(`${source}/`)) {
      return entry
    }
    if (target === undefined) {
      return { ...entry, invalid: true }
    }
    const path = target + entry.path.slice(source.length)
    return { ...entry, path, name: path.split('/').at(-1)!, invalid: false }
  }) }
}

export async function referencePrompt(modId: string): Promise<{ text: string, entries: SourceReference[] }> {
  const entries = [...(sourceReferences.value[modId] ?? [])]
  for (const entry of entries) {
    if (entry.invalid) {
      throw new Error(`文件引用已失效，请移除：${entry.path}`)
    }
    await workspaceService.entryAction({ modId, path: entry.path, action: 'reference' })
  }
  return { entries, text: entries.length ? `\n\n用户引用的当前 MOD 工作目录内路径（目录按需读取）：\n${entries.map(entry => `- ${entry.kind === 'directory' ? '目录' : '文件'} ${JSON.stringify(entry.path)}`).join('\n')}` : '' }
}
