<script setup lang="ts">
import { computed, defineAsyncComponent, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { useSourceBrowser } from './useSourceBrowser'
import { useSourceFileActions } from './useSourceFileActions'

const props = defineProps<{ modId: string, mode: 'move' | 'create', source: string, initialDirectory: string, blockedReason: string }>()
const emit = defineEmits<{ moved: [path: string], created: [path: string] }>()
const open = defineModel<boolean>('open', { default: false })
const SourceFileTree = defineAsyncComponent(() => import('./SourceFileTree.vue'))
const revision = shallowRef(0)
const targetName = shallowRef('')
const folderName = shallowRef('')
let generation = 0
const browser = useSourceBrowser({
  get modId() { return props.modId },
  get revision() { return revision.value },
  get active() { return open.value },
})
const actions = useSourceFileActions(() => props.modId)
const { busy, error, results } = actions
const { selected, directory, rootLoading, rootError } = browser
const directories = computed(() => browser.rows.value.filter(row => row.kind === 'directory'))
const failures = computed(() => results.value.filter(item => item.status !== 'moved'))
watch(() => [open.value, props.modId, props.mode], () => {
  generation++
  if (open.value) {
    folderName.value = ''
    targetName.value = props.source.split('/').at(-1) ?? ''
    error.value = ''
    results.value = []
    browser.selectDirectory(props.initialDirectory)
  }
}, { immediate: true, flush: 'sync' })

async function createFolder(close: boolean): Promise<void> {
  if (props.blockedReason || busy.value || !folderName.value.trim()) {
    return
  }
  const current = generation
  const value = await actions.createDirectory(directory.value, folderName.value)
  if (value && current === generation) {
    emit('created', value.path)
    folderName.value = ''
    await browser.refresh()
    if (current !== generation) {
      return
    }
    browser.selectDirectory(value.path)
    if (close) {
      open.value = false
    }
  }
}

async function submit(): Promise<void> {
  if (props.blockedReason || busy.value) {
    return
  }
  if (props.mode === 'create') {
    await createFolder(true)
    return
  }
  const current = generation
  await actions.moveFile(props.source, directory.value, targetName.value)
  if (current !== generation) {
    return
  }
  const moved = results.value.find(item => item.status === 'moved')
  if (moved) {
    emit('moved', moved.target)
    open.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="flex max-h-[85dvh] flex-col sm:max-w-xl">
      <DialogHeader>
        <DialogTitle>{{ mode === 'move' ? '移动或改名' : '新建文件夹' }}</DialogTitle>
        <DialogDescription>{{ mode === 'move' ? '只在当前项目内移动，源码中的文件引用不会自动修改。' : '在当前项目内创建单层目录。' }}</DialogDescription>
      </DialogHeader>
      <p class="break-all text-sm text-muted-foreground">
        目标目录：source/{{ directory }}
      </p>
      <SourceFileTree class="min-h-24 max-h-64" :rows="directories" :selected="selected" :selected-directory="directory" :loading="rootLoading" :error="rootError" @select-directory="browser.selectDirectory" @toggle="browser.toggle" @retry="browser.loadDirectory" />
      <FieldGroup>
        <Field v-if="mode === 'move'">
          <FieldLabel for="source-move-name">
            目标文件名
          </FieldLabel>
          <Input id="source-move-name" v-model="targetName" :disabled="busy || !!blockedReason" />
        </Field>
        <Field>
          <FieldLabel for="source-folder-name">
            新文件夹名称
          </FieldLabel>
          <div class="flex gap-2">
            <Input id="source-folder-name" v-model="folderName" :disabled="busy || !!blockedReason" @keydown.enter.prevent="mode === 'create' ? submit() : createFolder(false)" />
            <Button v-if="mode === 'move'" variant="outline" :disabled="busy || !folderName.trim() || !!blockedReason" @click="createFolder(false)">
              创建并选择
            </Button>
          </div>
        </Field>
        <FieldError v-if="blockedReason || error" :errors="[{ message: blockedReason || error }]" />
        <FieldError v-for="item in failures" :key="item.target" :errors="[{ message: item.message ?? '文件未移动' }]" />
      </FieldGroup>
      <DialogFooter>
        <Button variant="outline" :disabled="busy" @click="open = false">
          取消
        </Button>
        <Button :disabled="busy || !!blockedReason || (mode === 'move' ? !targetName.trim() : !folderName.trim())" @click="submit">
          {{ mode === 'move' ? '确认移动/改名' : '创建文件夹' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
