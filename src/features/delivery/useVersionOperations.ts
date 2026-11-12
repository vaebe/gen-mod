import type { VersionAction } from './VersionActions.vue'
import type { ModProject } from '@/contracts/mod'
import { computed, onScopeDispose, shallowRef, watch } from 'vue'
import { modService } from '@/features/projects/service'
import { deliveryService } from './service'

interface VersionOperationOptions {
  project: () => ModProject | undefined
  afterDelete: 'latest' | 'clear'
  updateProject: (project: ModProject) => void
}

/** 两个入口共享版本操作；项目数据更新始终归原项目，视图结果只归发起时视图。 */
export function useVersionOperations(options: VersionOperationOptions) {
  const busy = shallowRef(false)
  const notice = shallowRef('')
  const error = shallowRef('')
  const selectedId = shallowRef('')
  const deleting = shallowRef(false)
  const installing = shallowRef(false)
  const detailsOpen = shallowRef(false)
  const targetProject = shallowRef<ModProject>()
  const selected = computed(() => targetProject.value?.versions.find(version => version.versionId === selectedId.value))
  const actionsBusy = computed(() => busy.value || deleting.value || installing.value)
  let generation = 0
  watch(() => options.project()?.modId, () => {
    ++generation
    busy.value = false
    notice.value = ''
    error.value = ''
    selectedId.value = ''
    deleting.value = false
    installing.value = false
    detailsOpen.value = false
    targetProject.value = undefined
  }, { flush: 'sync' })
  watch(options.project, (project) => {
    if (!project || targetProject.value?.modId !== project.modId) {
      return
    }
    targetProject.value = project
    if (!project.versions.some(version => version.versionId === selectedId.value)) {
      selectedId.value = options.afterDelete === 'latest' ? project.latestVersionId ?? '' : ''
      detailsOpen.value = false
      deleting.value = false
      installing.value = false
    }
  }, { flush: 'sync' })
  onScopeDispose(() => {
    ++generation
  })

  async function act(kind: VersionAction, versionId: string): Promise<void> {
    const project = options.project()
    if (!project || actionsBusy.value || !project.versions.some(version => version.versionId === versionId)) {
      return
    }
    targetProject.value = project
    selectedId.value = versionId
    notice.value = ''
    error.value = ''
    if (kind === 'delete') {
      deleting.value = true
      return
    }
    if (kind === 'details') {
      detailsOpen.value = true
      return
    }
    if (kind === 'install') {
      installing.value = true
      return
    }
    const current = generation
    busy.value = true
    try {
      if (kind === 'export') {
        const path = await deliveryService.exportVersion(project.modId, versionId)
        if (current === generation && path) {
          notice.value = `已导出：${path}`
        }
      }
      else { await deliveryService.openDirectory(project.modId, versionId) }
    }
    catch (cause) {
      if (current === generation) {
        error.value = (cause as Error).message
      }
    }
    finally {
      if (current === generation) {
        busy.value = false
      }
    }
  }
  function acceptDeletion(result: ModProject, current: number, succeeded: boolean): void {
    options.updateProject(result)
    if (current !== generation) {
      return
    }
    targetProject.value = result
    if (succeeded || !result.versions.some(version => version.versionId === selectedId.value)) {
      selectedId.value = options.afterDelete === 'latest' ? result.latestVersionId ?? '' : ''
      detailsOpen.value = false
    }
  }
  async function confirmDelete(): Promise<void> {
    const project = targetProject.value
    const versionId = selectedId.value
    if (!deleting.value || !project || !selected.value || busy.value) {
      return
    }
    const current = generation
    busy.value = true
    error.value = ''
    try {
      const result = await deliveryService.deleteVersion(project.modId, versionId)
      acceptDeletion(result, current, true)
      if (current === generation) {
        notice.value = '版本已删除，已部署文件和安装记录保留。'
      }
    }
    catch (cause) {
      if (current === generation) {
        error.value = (cause as Error).message
      }
      // 删除记录提交后仍可能因目录清理失败报错，必须重新读取原项目。
      try {
        const actual = await modService.get(project.modId)
        acceptDeletion(actual, current, false)
      }
      catch (refreshCause) {
        if (current === generation) {
          error.value += `；重新读取版本失败：${(refreshCause as Error).message}`
        }
      }
    }
    finally {
      if (current === generation) {
        deleting.value = false
        busy.value = false
      }
    }
  }
  return { act, confirmDelete, busy, notice, error, selectedId, selected, deleting, installing, detailsOpen, targetProject, actionsBusy }
}
