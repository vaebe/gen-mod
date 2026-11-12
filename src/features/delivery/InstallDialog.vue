<script setup lang="ts">
import type { InstallHint, InstallPreview } from '@/contracts/install'
import { shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { displayPath } from '@/lib/displayPath'
import { installationService } from './service'
import { useDeliveriesStore } from './useDeliveries'

const props = defineProps<{ modId: string, versionId: string, previousTarget?: string }>()
const emit = defineEmits<{ targetSelected: [directory: string] }>()
const deliveries = useDeliveriesStore()
const open = defineModel<boolean>('open', { default: false })
const preview = shallowRef<InstallPreview>()
const busy = shallowRef(false)
const error = shallowRef('')
const directoryHint = shallowRef<InstallHint>()
const hintError = shallowRef('')
watch(() => [open.value, props.modId, props.versionId], async (_value, _previous, onCleanup) => {
  let current = true
  onCleanup(() => {
    current = false
  })
  preview.value = undefined
  error.value = ''
  directoryHint.value = undefined
  hintError.value = ''
  if (!open.value) {
    return
  }
  try {
    const hint = await installationService.hint(props.modId, props.versionId)
    if (current) {
      directoryHint.value = hint
    }
  }
  catch (cause) {
    if (current) {
      hintError.value = `目录说明读取失败：${(cause as Error).message}`
    }
  }
}, { immediate: true })
// 即使弹窗因切换项目被卸载，已发生的安装写入仍刷新原项目。
async function refreshProject(modId: string): Promise<void> {
  try {
    await deliveries.refresh(modId)
  }
  catch (cause) { error.value += `${error.value ? '；' : ''}重新读取安装状态失败：${(cause as Error).message}` }
}
async function choose(previousTarget?: string): Promise<void> {
  const modId = props.modId
  const versionId = props.versionId
  busy.value = true
  error.value = ''
  preview.value = undefined
  try {
    const target = previousTarget ?? await installationService.pickDirectory()
    if (target) {
      preview.value = await installationService.preview(modId, versionId, target)
      emit('targetSelected', preview.value.targetDirectory)
      await refreshProject(modId)
    }
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
async function install(): Promise<void> {
  if (!preview.value) {
    return
  }
  const modId = props.modId
  busy.value = true
  error.value = ''
  try {
    const result = await installationService.install(preview.value.previewId)
    await refreshProject(modId)
    if (result.status === 'succeeded') {
      open.value = false
    }
    else { error.value = `${result.error ?? '安装失败'}${result.rollback === 'notAttempted' ? '；文件可能已部分覆盖，本次没有备份，请检查目标目录。' : ''}` }
    preview.value = undefined
  }
  catch (cause) {
    error.value = (cause as Error).message
    preview.value = undefined
    await refreshProject(modId)
  }
  finally { busy.value = false }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="max-w-2xl" @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()">
      <DialogHeader><DialogTitle>安装选定版本</DialogTitle><DialogDescription>版本 {{ versionId.slice(0, 8) }}。安装会直接覆盖清单中的同名文件，不创建备份。</DialogDescription></DialogHeader>
      <div v-if="directoryHint" class="flex flex-col gap-1 text-sm">
        <p>{{ directoryHint.description }}</p>
        <p v-if="directoryHint.examplePath" class="break-all text-xs text-muted-foreground">
          路径示例：{{ directoryHint.examplePath }}。文件将按下方预览放入此目录。
        </p>
      </div>
      <p v-if="hintError" role="alert" class="text-sm text-destructive">
        {{ hintError }}
      </p>
      <div v-if="previousTarget && !preview" class="space-y-3 rounded-lg border bg-muted/30 p-4">
        <p class="text-sm">
          使用上次的部署位置，还是重新选择？
        </p><p class="break-all text-xs text-muted-foreground">
          {{ displayPath(previousTarget) }}
        </p><div class="flex flex-wrap justify-end gap-2">
          <Button :disabled="busy" @click="choose(previousTarget)">
            使用之前的位置
          </Button><Button variant="outline" :disabled="busy" @click="choose()">
            重新选择
          </Button>
        </div>
      </div>
      <Button v-else class="justify-self-end" variant="outline" :disabled="busy" @click="choose()">
        {{ preview ? '重新选择目标目录' : '选择目标目录' }}
      </Button>
      <template v-if="preview">
        <p class="break-all text-xs">
          {{ displayPath(preview.targetDirectory) }}
        </p><div class="max-h-72 space-y-3 overflow-auto rounded-md border p-3">
          <div v-for="entry in preview.entries" :key="entry.target" class="text-xs">
            <p class="break-all font-mono">
              {{ displayPath(entry.source) }}
            </p><p class="mt-1 break-all font-mono">
              → {{ displayPath(entry.target) }}
            </p><p class="mt-1 text-muted-foreground">
              {{ entry.overwrite ? '覆盖已有文件' : '新增文件' }}
            </p>
          </div>
        </div><Button class="justify-self-end" :disabled="busy" @click="install">
          {{ busy ? '正在安装…' : '确认安装' }}
        </Button>
      </template>
      <p v-if="error" class="text-sm text-destructive" role="alert">
        {{ error }}
      </p>
    </DialogContent>
  </Dialog>
</template>
