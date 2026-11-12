import type { ModProject } from '@/contracts/mod'
import { defineStore } from 'pinia'
import { reactive, shallowRef } from 'vue'
import { modService } from '@/features/projects/service'
import { useProjectsStore } from '@/stores/projects'

/** 交付及安装共用项目快照；读取代次按项目隔离，后台更新不依赖视图。 */
export const useDeliveriesStore = defineStore('deliveries', () => {
  const projects = useProjectsStore()
  const snapshots = shallowRef<Record<string, ModProject>>({})
  const errors = shallowRef<Record<string, string>>({})
  const revisions = reactive(new Map<string, number>())
  const generations = new Map<string, number>()
  function projectFor(modId: string): ModProject | undefined {
    const snapshot = snapshots.value[modId]
    const current = projects.projects.find(project => project.modId === modId)
    return current && (!snapshot || current.revision >= snapshot.revision) ? current : snapshot
  }
  const errorFor = (modId: string) => errors.value[modId] ?? ''
  const revisionFor = (modId: string) => revisions.get(modId) ?? 0
  function accept(project: ModProject): void {
    if (projects.isRemoved(project.modId) || (projectFor(project.modId)?.revision ?? -1) > project.revision) {
      return
    }
    snapshots.value = { ...snapshots.value, [project.modId]: project }
    errors.value = { ...errors.value, [project.modId]: '' }
    projects.upsert(project)
  }
  function update(project: ModProject): void {
    generations.set(project.modId, (generations.get(project.modId) ?? 0) + 1)
    accept(project)
    revisions.set(project.modId, revisionFor(project.modId) + 1)
  }
  async function refresh(modId: string): Promise<ModProject | undefined> {
    if (!modId) {
      return
    }
    const epoch = (generations.get(modId) ?? 0) + 1
    generations.set(modId, epoch)
    revisions.set(modId, revisionFor(modId) + 1)
    try {
      const project = await modService.get(modId)
      if (generations.get(modId) === epoch) {
        accept(project)
      }
      return projectFor(modId)
    }
    catch (cause) {
      if (generations.get(modId) !== epoch) {
        return projectFor(modId)
      }
      errors.value = { ...errors.value, [modId]: (cause as Error).message }
      throw cause
    }
  }
  function forget(modId: string): void {
    generations.set(modId, (generations.get(modId) ?? 0) + 1)
    const next = { ...snapshots.value }
    delete next[modId]
    snapshots.value = next
    errors.value = { ...errors.value, [modId]: '' }
  }
  return { projectFor, errorFor, revisionFor, refresh, update, forget }
})
