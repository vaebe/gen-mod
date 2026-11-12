import type { ModProject } from '@/contracts/mod'
import { defineStore } from 'pinia'
import { shallowRef } from 'vue'
import { modService } from '@/features/projects/service'

/** 跨页面共享项目列表，读取失败不能伪装成空项目。 */
export const useProjectsStore = defineStore('projects', () => {
  const projects = shallowRef<ModProject[]>([])
  const error = shallowRef('')
  const loading = shallowRef(false)
  const notice = shallowRef('')
  const invalidIds = new Set<string>()
  let generation = 0
  async function refresh(): Promise<void> {
    const epoch = ++generation
    loading.value = true
    error.value = ''
    try {
      const [values, recoveries] = await Promise.all([modService.list(), modService.recoveryIssues()])
      if (epoch !== generation) {
        return
      }
      projects.value = values.filter(project => !project.archived)
      if (recoveries.length) {
        error.value = recoveries.map(item => `文件操作需核对 ${item.modId}：${item.kind}`).join('；')
      }
    }
    catch (cause) {
      if (epoch === generation) {
        error.value = (cause as Error).message
      }
    }
    finally {
      if (epoch === generation) {
        loading.value = false
      }
    }
  }
  function upsert(project: ModProject): void {
    if (invalidIds.has(project.modId)) {
      return
    }
    const previous = projects.value.find(item => item.modId === project.modId)
    if (previous && previous.revision > project.revision) {
      return
    }
    projects.value = project.archived ? projects.value.filter(item => item.modId !== project.modId) : [project, ...projects.value.filter(item => item.modId !== project.modId)]
  }
  function removeLocal(modId: string): void {
    ++generation
    invalidIds.add(modId)
    projects.value = projects.value.filter(item => item.modId !== modId)
    loading.value = false
  }
  function restoreLocal(project: ModProject): void {
    invalidIds.delete(project.modId)
    upsert(project)
  }
  function isRemoved(id: string): boolean {
    return invalidIds.has(id)
  }
  return { projects, error, loading, notice, refresh, upsert, removeLocal, restoreLocal, isRemoved }
})
