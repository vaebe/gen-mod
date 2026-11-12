<script setup lang="ts">
import { FilePlus, FolderPlus, Maximize2, Minimize2, RefreshCw, Upload } from '@lucide/vue'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { computed, defineAsyncComponent, onScopeDispose, shallowRef, watch } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import { Button } from '@/components/ui/button'
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '@/components/ui/resizable'
import { Textarea } from '@/components/ui/textarea'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import { useSourceManager } from './useSourceManager'

const props = defineProps<{ modId: string, revision: number, active: boolean, readOnly?: boolean }>()
const SourceExplorer = defineAsyncComponent(() => import('./SourceExplorer.vue'))
const SourceFilePreview = defineAsyncComponent(() => import('./SourceFilePreview.vue'))
const SourceDecisionDialog = defineAsyncComponent(() => import('./SourceDecisionDialog.vue'))
const enlarged = shallowRef(false)
const session = useSessionStore()
const blockedReason = computed(() => {
  if (props.readOnly) {
    return '项目已归档，恢复后可修改文件。'
  }
  return session.isProjectBusy(props.modId) ? '当前项目有活动 AI 任务，请结束后操作文件。' : ''
})
const manager = useSourceManager(props, () => blockedReason.value)
const { browser, selection, busy, error, results, editing, editError, text, dirty, editPath, question } = manager
const { rows, selected, preview, previewLoading, previewError, rootLoading, rootError } = browser
const disabled = computed(() => busy.value || !!blockedReason.value || !props.active)
const tools = [{ action: 'file', label: '新建文件', icon: FilePlus }, { action: 'folder', label: '新建文件夹', icon: FolderPlus }, { action: 'import', label: '导入文件', icon: Upload }] as const
const statuses = computed(() => {
  const result: Record<string, string> = {}
  if (session.modId !== props.modId || !session.task) {
    return result
  }
  const taskId = session.task.taskId
  const calls = new Set(session.chat.messages.filter(message => message.taskId === taskId).flatMap(message => message.parts.filter(part => part.type === 'tool').map(part => part.toolCallId)))
  const root = session.task.sourceDirectory.replaceAll('\\', '/').replace(/\/$/u, '')
  for (const tool of Object.values(session.chat.tools)) {
    if (!calls.has(tool.toolCallId) || !['edit', 'write'].includes(tool.name) || !tool.args || typeof tool.args !== 'object') {
      continue
    }
    const args = tool.args as Record<string, unknown>
    let path = typeof args.path === 'string' ? args.path.replaceAll('\\', '/') : ''
    if (path.toLowerCase().startsWith(`${root.toLowerCase()}/`)) {
      path = path.slice(root.length + 1)
    }
    if (!path || path.startsWith('/') || /^[a-z]:/iu.test(path) || path.split('/').includes('..')) {
      continue
    }
    result[path] = tool.status === 'running' ? '修改中' : tool.status === 'succeeded' ? '已修改' : tool.status === 'failed' ? '修改失败' : ''
  }
  return result
})
watch(() => props.modId, () => {
  enlarged.value = false
})
onBeforeRouteLeave(() => manager.leave())
onBeforeRouteUpdate(() => manager.leave())

let disposed = false
const stops: (() => void)[] = []
function subscribe(promise: Promise<() => void>): void {
  void promise.then((stop) => {
    if (disposed) {
      stop()
    }
    else {
      stops.push(stop)
    }
  }).catch((cause) => {
    if (!disposed) {
      error.value = `文件事件监听失败：${String(cause)}`
    }
  })
}
subscribe(listen<{ modId: string }>('project-source-changed', ({ payload }) => {
  if (payload.modId === props.modId && props.active && !busy.value) {
    void browser.refresh()
  }
}))
subscribe(getCurrentWebviewWindow().onDragDropEvent(({ payload }) => {
  if (payload.type !== 'drop' || disabled.value) {
    return
  }
  const point = payload.position.toLogical(window.devicePixelRatio)
  const hit = document.elementFromPoint(point.x, point.y)
  const target = hit?.closest<HTMLElement>('[data-source-directory]')
  if (target?.closest<HTMLElement>('[data-mod-tree]')?.dataset.modTree === props.modId && target.dataset.sourceDirectory !== undefined) {
    void manager.importPaths(payload.paths, target.dataset.sourceDirectory)
  }
}))
subscribe(getCurrentWebviewWindow().onCloseRequested(async (event) => {
  if (!dirty.value && !busy.value) {
    return
  }
  event.preventDefault()
  if (await manager.leave()) {
    try {
      await getCurrentWebviewWindow().destroy()
    }
    catch (cause) {
      error.value = `关闭窗口失败：${String(cause)}`
    }
  }
}))
onScopeDispose(() => {
  disposed = true
  stops.forEach(stop => stop())
})
</script>

<template>
  <section aria-label="MOD 文件管理" class="flex min-h-0 flex-1 flex-col bg-background" :class="enlarged && 'fixed inset-4 z-40 rounded-lg border shadow-xl'">
    <div class="flex shrink-0 items-center gap-1 border-b p-2">
      <span class="mr-auto text-xs text-muted-foreground">工作文件</span>
      <Button v-for="tool in tools" :key="tool.action" size="icon-sm" variant="ghost" :title="tool.label" :aria-label="tool.label" :disabled="disabled || !!editing" @click="manager.action(tool.action)">
        <component :is="tool.icon" />
      </Button>
      <Button size="icon-sm" variant="ghost" title="刷新" aria-label="刷新文件" :disabled="rootLoading" @click="browser.refresh">
        <RefreshCw :class="rootLoading && 'animate-spin'" />
      </Button>
      <Button size="icon-sm" variant="ghost" :aria-label="enlarged ? '退出放大' : '放大文件管理'" @click="enlarged = !enlarged">
        <Minimize2 v-if="enlarged" /><Maximize2 v-else />
      </Button>
    </div>
    <p v-if="blockedReason" role="status" class="px-3 py-2 text-xs text-muted-foreground">
      {{ blockedReason }}
    </p>
    <p v-if="error" role="alert" class="break-all px-3 py-2 text-xs text-destructive">
      {{ error }}
    </p>
    <div v-if="results.length" class="max-h-28 shrink-0 overflow-auto border-b px-3 py-2 text-xs" role="status">
      <p>{{ results.filter(item => ['moved', 'copied', 'deleted'].includes(item.status)).length }} 项完成 · {{ results.filter(item => item.status === 'skipped').length }} 项跳过 · {{ results.filter(item => item.status === 'failed' || item.status === 'conflict').length }} 项未完成</p>
      <p v-for="(item, index) in results.filter(item => item.message)" :key="index" class="break-all">
        {{ item.source }}：{{ item.message }}
      </p>
      <Button v-if="manager.retryOperation.value && results.some(item => item.status === 'failed' || item.status === 'conflict')" size="sm" variant="outline" :disabled="disabled" @click="manager.retryFailed">
        重试未完成项
      </Button>
    </div>
    <ResizablePanelGroup direction="horizontal" class="min-h-0 flex-1">
      <ResizablePanel :default-size="35" :min-size="20">
        <SourceExplorer v-model:selection="selection" :mod-id="modId" :rows="rows" :loading="rootLoading" :error="rootError" :disabled="disabled" :editing="editing" :edit-error="editError" :statuses="statuses" @open="manager.open" @toggle="browser.toggle" @action="manager.action" @move="manager.move" @name="manager.updateName" @submit="manager.submitName" @cancel="!busy && (editing = undefined)" />
      </ResizablePanel>
      <ResizableHandle with-handle />
      <ResizablePanel :default-size="65" :min-size="25">
        <div v-if="editPath" class="flex h-full min-h-0 flex-col" @keydown.ctrl.s.prevent="manager.save">
          <div class="flex shrink-0 items-center gap-2 border-b p-2">
            <span class="min-w-0 flex-1 truncate font-mono text-xs" :title="editPath">{{ editPath }}{{ dirty ? ' • 未保存' : '' }}</span><Button size="sm" :disabled="disabled || !dirty" @click="manager.save">
              保存
            </Button>
            <Button size="sm" variant="ghost" :disabled="busy" @click="manager.reload">
              重新加载
            </Button>
          </div>
          <Textarea v-model="text" :disabled="disabled" spellcheck="false" aria-label="文件内容" class="min-h-0 flex-1 resize-none rounded-none border-0 font-mono text-xs" />
        </div>
        <SourceFilePreview v-else class="h-full" :mod-id="modId" :path="selected" :preview="preview" :loading="previewLoading" :error="previewError" />
      </ResizablePanel>
    </ResizablePanelGroup>
    <SourceDecisionDialog :question="question" @answer="manager.answer" />
  </section>
</template>
