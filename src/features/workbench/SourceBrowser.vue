<script setup lang="ts">
import type { SourceImportItem } from '@/contracts/source'
import { ArrowRightLeft, FolderPlus, Maximize2, RefreshCw, Upload } from '@lucide/vue'
import { listen } from '@tauri-apps/api/event'
import { computed, defineAsyncComponent, onScopeDispose, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { nativeCall } from '@/services/native'
import { useProjectsStore } from '@/stores/projects'
import { useSessionStore } from '@/stores/session'
import SourceFilePreview from './SourceFilePreview.vue'
import SourceFileTree from './SourceFileTree.vue'
import { useSourceBrowser } from './useSourceBrowser'
import { useSourceFileActions } from './useSourceFileActions'

const props = defineProps<{ modId: string, revision: number, active: boolean, readOnly?: boolean }>()
const SourceDirectoryDialog = defineAsyncComponent(() => import('./SourceDirectoryDialog.vue'))
const LegacyAssetsRecovery = defineAsyncComponent(() => import('./LegacyAssetsRecovery.vue'))
const legacyOpen = shallowRef(false)
const legacyCount = shallowRef(0)
const legacyError = shallowRef('')
const enlarged = shallowRef(false)
const directoryOpen = shallowRef(false)
const directoryMode = shallowRef<'move' | 'create'>('move')
const pending = shallowRef<SourceImportItem[]>([])
const importDirectory = shallowRef('')
const reviewOpen = shallowRef(false)
const selecting = shallowRef(false)
const notice = shallowRef('')
let generation = 0
const projects = useProjectsStore()
const session = useSessionStore()
const blockedReason = computed(() => {
  if (props.readOnly) {
    return '项目已归档，恢复后可操作文件。'
  }
  const project = projects.projects.find(item => item.modId === props.modId)
  const busy = project?.sessions.some(item => session.isBusy(props.modId, item.sessionId) || session.isRunning(props.modId, item.sessionId))
    || (session.modId === props.modId && (session.busy || session.working || session.compacting))
  return busy ? '当前项目有活动 AI 任务，请结束后操作文件。' : ''
})
const actions = useSourceFileActions(() => props.modId)
const { busy, error: actionError, results } = actions
const failures = computed(() => results.value.filter(item => item.status !== 'imported'))
const disabled = computed(() => busy.value || selecting.value || !!blockedReason.value)
const { rows, selected, directory, preview, previewLoading, previewError, rootLoading, rootError, toggle, select, selectDirectory, reveal, refresh, loadDirectory } = useSourceBrowser(props)
const fileActions = [{ kind: 'import', label: '导入文件', icon: Upload }, { kind: 'move', label: '移动/改名', icon: ArrowRightLeft }, { kind: 'create', label: '新建文件夹', icon: FolderPlus }] as const
watch(() => props.modId, () => {
  generation++
  enlarged.value = false
  directoryOpen.value = false
  legacyOpen.value = false
  legacyCount.value = 0
  legacyError.value = ''
  reviewOpen.value = false
  selecting.value = false
  pending.value = []
  notice.value = ''
}, { flush: 'sync' })
async function refreshLegacy(): Promise<void> {
  if (!props.active || !props.modId) {
    return
  }
  const current = generation
  try {
    const assets = await nativeCall<{ restored: boolean }[]>('list_legacy_assets', { modId: props.modId })
    if (current === generation) {
      legacyCount.value = assets.filter(asset => !asset.restored).length
      legacyError.value = ''
    }
  }
  catch (cause) {
    if (current === generation) {
      legacyError.value = (cause as Error).message
    }
  }
}
watch(() => [props.modId, props.active, props.revision], () => {
  void refreshLegacy()
}, { immediate: true })
async function restored(path: string): Promise<void> {
  const current = generation
  await refresh()
  if (current !== generation) {
    return
  }
  await reveal(path)
  await refreshLegacy()
}

async function importPending(): Promise<void> {
  if (disabled.value || !pending.value.length) {
    return
  }
  const current = generation
  notice.value = ''
  await actions.importFiles(importDirectory.value, pending.value)
  if (current !== generation) {
    return
  }
  const imported = results.value.filter(item => item.status === 'imported')
  pending.value = pending.value.filter(item => !imported.some(result => result.source === item.sourcePath
    && result.target === (importDirectory.value ? `${importDirectory.value}/${item.targetName}` : item.targetName)))
  notice.value = imported.length ? `已原样导入 ${imported.length} 个文件。` : ''
  reviewOpen.value = pending.value.length > 0
  await refresh()
}

async function perform(kind: 'import' | 'move' | 'create'): Promise<void> {
  if (disabled.value) {
    return
  }
  if (kind !== 'import') {
    directoryMode.value = kind
    directoryOpen.value = true
    return
  }
  const current = generation
  const target = directory.value
  selecting.value = true
  actionError.value = ''
  try {
    const files = await nativeCall<string[] | null>('pick_source_import_files')
    if (current !== generation || !files?.length) {
      return
    }
    pending.value = files.map(sourcePath => ({ sourcePath, targetName: sourcePath.split(/[\\/]/).at(-1) ?? '' }))
    importDirectory.value = target
    selecting.value = false
    await importPending()
  }
  catch (cause) {
    if (current === generation) {
      actionError.value = (cause as Error).message
    }
  }
  finally {
    if (current === generation) {
      selecting.value = false
    }
  }
}

function renamePending(index: number, value: string | number): void {
  pending.value = pending.value.map((item, itemIndex) => itemIndex === index ? { ...item, targetName: String(value) } : item)
}

async function moved(path: string): Promise<void> {
  const current = generation
  notice.value = `已移动到 source/${path}。`
  await refresh()
  if (current === generation) {
    await reveal(path)
  }
}

let disposed = false
let unlisten: (() => void) | undefined
void listen<{ modId: string, sourceRevision: number }>('project-source-changed', ({ payload }) => {
  // inactive 时由 useSourceBrowser 的 active watcher 在重新展示时刷新。
  if (payload.modId === props.modId && props.active) {
    void refresh()
  }
}).then((stop) => {
  if (disposed) {
    stop()
  }
  else {
    unlisten = stop
  }
}).catch((cause) => {
  if (!disposed) {
    actionError.value = `文件变更监听失败：${String(cause)}`
  }
})
onScopeDispose(() => {
  disposed = true
  generation++
  unlisten?.()
})

/** 根目录重试需一并恢复已展开分支；子目录只重试当前层。 */
function retry(path: string): void {
  if (path) {
    void loadDirectory(path)
  }
  else { void refresh() }
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <p v-if="!modId" class="p-4 text-xs text-muted-foreground">
      选择项目后浏览源码。
    </p>
    <template v-else>
      <div class="flex shrink-0 items-center justify-between border-b px-3 py-2">
        <span class="font-mono text-[11px] font-medium text-muted-foreground">source</span>
        <TooltipProvider :delay-duration="150">
          <div class="flex items-center gap-1">
            <Tooltip v-for="action in fileActions" :key="action.kind">
              <TooltipTrigger as-child>
                <span v-if="disabled || (action.kind === 'move' && !selected)" tabindex="0" :aria-label="action.label" class="inline-flex">
                  <Button variant="ghost" size="icon-sm" disabled :aria-label="action.label">
                    <component :is="action.icon" aria-hidden="true" />
                  </Button>
                </span>
                <Button v-else variant="ghost" size="icon-sm" :aria-label="action.label" @click="perform(action.kind)">
                  <component :is="action.icon" aria-hidden="true" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{{ action.label }}</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger as-child>
                <span v-if="rootLoading" tabindex="0" aria-label="刷新源码文件" class="inline-flex">
                  <Button size="icon-sm" variant="ghost" aria-label="刷新源码文件" disabled>
                    <RefreshCw class="animate-spin" aria-hidden="true" />
                  </Button>
                </span>
                <Button v-else size="icon-sm" variant="ghost" aria-label="刷新源码文件" @click="refresh">
                  <RefreshCw aria-hidden="true" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>刷新源码文件</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger as-child>
                <Button size="icon-sm" variant="ghost" aria-label="放大源码浏览器" @click="enlarged = true">
                  <Maximize2 aria-hidden="true" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>放大预览</TooltipContent>
            </Tooltip>
          </div>
        </TooltipProvider>
      </div>
      <div v-if="pending.length" class="flex shrink-0 flex-wrap gap-1 border-b px-3 py-2">
        <Button variant="ghost" size="sm" @click="reviewOpen = true">
          待处理 {{ pending.length }}
        </Button>
      </div>
      <p v-if="blockedReason || actionError || notice" class="break-words px-3 py-2 text-xs text-muted-foreground" role="status">
        {{ blockedReason || actionError || notice }}
      </p>
      <ResizablePanelGroup v-if="!enlarged" direction="vertical" class="min-h-0 flex-1">
        <ResizablePanel :default-size="40" :min-size="20">
          <SourceFileTree class="h-full" :rows="rows" :selected="selected" :selected-directory="directory" :loading="rootLoading" :error="rootError" @select="select" @select-directory="selectDirectory" @toggle="toggle" @retry="retry" />
        </ResizablePanel>
        <ResizableHandle with-handle />
        <ResizablePanel :default-size="60" :min-size="25">
          <SourceFilePreview class="h-full" :mod-id="modId" :path="selected" :preview="preview" :loading="previewLoading" :error="previewError" />
        </ResizablePanel>
      </ResizablePanelGroup>
      <Dialog v-model:open="enlarged">
        <DialogContent class="flex h-[85dvh] w-[calc(100vw-2rem)] max-w-none flex-col gap-0 overflow-hidden p-0 sm:max-w-[1400px]">
          <DialogHeader class="shrink-0 border-b px-5 py-4 pr-12">
            <DialogTitle>项目源码</DialogTitle><DialogDescription>展开文件夹，选择文件查看当前源码。预览为只读。</DialogDescription>
          </DialogHeader>
          <div class="flex shrink-0 items-center justify-between border-b px-4 py-2">
            <span class="font-mono text-xs text-muted-foreground">source</span>
            <TooltipProvider :delay-duration="150">
              <div class="flex items-center gap-1">
                <Tooltip v-for="action in fileActions" :key="action.kind">
                  <TooltipTrigger as-child>
                    <span v-if="disabled || (action.kind === 'move' && !selected)" tabindex="0" :aria-label="action.label" class="inline-flex">
                      <Button variant="ghost" size="icon-sm" disabled :aria-label="action.label">
                        <component :is="action.icon" aria-hidden="true" />
                      </Button>
                    </span>
                    <Button v-else variant="ghost" size="icon-sm" :aria-label="action.label" @click="perform(action.kind)">
                      <component :is="action.icon" aria-hidden="true" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{{ action.label }}</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <span v-if="rootLoading" tabindex="0" aria-label="刷新源码文件" class="inline-flex">
                      <Button size="icon-sm" variant="ghost" aria-label="刷新源码文件" disabled>
                        <RefreshCw class="animate-spin" aria-hidden="true" />
                      </Button>
                    </span>
                    <Button v-else size="icon-sm" variant="ghost" aria-label="刷新源码文件" @click="refresh">
                      <RefreshCw aria-hidden="true" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>刷新源码文件</TooltipContent>
                </Tooltip>
              </div>
            </TooltipProvider>
          </div>
          <div v-if="pending.length" class="flex shrink-0 flex-wrap gap-1 border-b px-4 py-2">
            <Button variant="ghost" size="sm" @click="reviewOpen = true">
              待处理 {{ pending.length }}
            </Button>
          </div>
          <p v-if="blockedReason || actionError || notice" class="break-words px-4 py-2 text-xs text-muted-foreground" role="status">
            {{ blockedReason || actionError || notice }}
          </p>
          <ResizablePanelGroup direction="horizontal" class="min-h-0 flex-1">
            <ResizablePanel :default-size="25" :min-size="15" :max-size="50">
              <SourceFileTree class="h-full" :rows="rows" :selected="selected" :selected-directory="directory" :loading="rootLoading" :error="rootError" @select="select" @select-directory="selectDirectory" @toggle="toggle" @retry="retry" />
            </ResizablePanel>
            <ResizableHandle with-handle />
            <ResizablePanel :default-size="75" :min-size="40">
              <SourceFilePreview class="h-full" :mod-id="modId" :path="selected" :preview="preview" :loading="previewLoading" :error="previewError" />
            </ResizablePanel>
          </ResizablePanelGroup>
        </DialogContent>
      </Dialog>
      <SourceDirectoryDialog v-model:open="directoryOpen" :mod-id="modId" :mode="directoryMode" :source="selected" :initial-directory="directory" :blocked-reason="blockedReason" @moved="moved" @created="refresh" />
      <Button v-if="legacyCount" variant="ghost" size="sm" :disabled="disabled" @click="legacyOpen = true">
        恢复旧素材（{{ legacyCount }}）
      </Button>
      <p v-if="legacyError" role="alert" class="p-3 text-xs text-destructive">
        旧素材恢复信息读取失败：{{ legacyError }}<Button variant="ghost" size="sm" @click="refreshLegacy">
          重试
        </Button>
      </p>
      <LegacyAssetsRecovery v-model:open="legacyOpen" :mod-id="modId" :blocked-reason="blockedReason" @restored="restored" />
      <Dialog v-model:open="reviewOpen">
        <DialogContent class="flex max-h-[85dvh] flex-col sm:max-w-xl">
          <DialogHeader><DialogTitle>处理未导入的文件</DialogTitle><DialogDescription>目标：source/{{ importDirectory }}。同名文件不会覆盖，可修改目标文件名后重试，或跳过。</DialogDescription></DialogHeader>
          <FieldGroup class="min-h-0 overflow-auto">
            <Field v-for="(item, index) in pending" :key="`${index}-${item.sourcePath}`">
              <FieldLabel :for="`source-import-${index}`">
                {{ item.sourcePath }}
              </FieldLabel>
              <div class="flex gap-2">
                <Input :id="`source-import-${index}`" :model-value="item.targetName" :disabled="disabled" @update:model-value="renamePending(index, $event)" /><Button variant="outline" :disabled="busy" @click="pending = pending.filter((_, itemIndex) => index !== itemIndex)">
                  跳过
                </Button>
              </div>
            </Field>
            <FieldError v-if="actionError || blockedReason" :errors="[{ message: actionError || blockedReason }]" />
            <FieldError v-for="(item, index) in failures" :key="index" :errors="[{ message: `${item.target}：${item.message ?? '未导入'}` }]" />
          </FieldGroup>
          <div class="flex justify-end gap-2">
            <Button variant="outline" :disabled="busy" @click="reviewOpen = false">
              关闭
            </Button><Button :disabled="disabled || !pending.length" @click="importPending">
              重试导入
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </template>
  </div>
</template>
