<script setup lang="ts">
import type { ModProject } from '@/contracts/mod'
import { defineAsyncComponent, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Empty, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { modService } from '@/services/native'
import { useProjectsStore } from '@/stores/projects'

const open = defineModel<boolean>('open', { required: true })
const RemoveProjectDialog = defineAsyncComponent(() => import('./RemoveProjectDialog.vue'))
const projects = useProjectsStore()
const archived = shallowRef<ModProject[]>([])
const removing = shallowRef<ModProject>()
const busy = shallowRef(false)
const error = shallowRef('')
let generation = 0
async function refresh(): Promise<void> {
  const epoch = ++generation
  busy.value = true
  error.value = ''
  try {
    const values = await modService.archived()
    if (epoch === generation) {
      archived.value = values
    }
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
watch(open, (value) => {
  if (value) {
    void refresh()
  }
})
async function restore(project: ModProject): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    projects.restoreLocal(await modService.archive(project.modId, project.revision, false))
    archived.value = archived.value.filter(item => item.modId !== project.modId)
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>已归档项目</DialogTitle>
        <DialogDescription>恢复后使用原来的项目、源码、记忆和会话。</DialogDescription>
      </DialogHeader>
      <p v-if="error" role="alert" class="text-sm text-destructive">
        {{ error }}
      </p>
      <Button v-if="error" variant="outline" :disabled="busy" @click="refresh">
        重新读取
      </Button>
      <p v-if="busy" role="status" class="text-sm text-muted-foreground">
        处理中…
      </p>
      <Empty v-if="!busy && !error && !archived.length">
        <EmptyHeader><EmptyTitle>没有已归档项目</EmptyTitle></EmptyHeader>
      </Empty>
      <ul class="flex max-h-[55vh] flex-col gap-3 overflow-auto">
        <li v-for="project in archived" :key="project.modId" class="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-3">
          <RouterLink :to="`/mods/${project.modId}/chat`" class="min-w-0 flex-1 truncate underline-offset-4 hover:underline" @click="open = false">
            {{ project.name }}（只读）
          </RouterLink>
          <div class="flex gap-2">
            <Button variant="outline" size="sm" :disabled="busy" @click="restore(project)">
              恢复
            </Button>
            <Button variant="ghost" size="sm" :disabled="busy" @click="removing = project">
              永久删除
            </Button>
          </div>
        </li>
      </ul>
    </DialogContent>
  </Dialog>
  <RemoveProjectDialog v-model="removing" @removed="refresh" />
</template>
