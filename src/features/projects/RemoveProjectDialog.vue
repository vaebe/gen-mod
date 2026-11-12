<script setup lang="ts">
import type { ModProject } from '@/contracts/mod'
import { computed, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { modService } from '@/features/projects/service'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import { useProjectsStore } from '@/stores/projects'

const emit = defineEmits<{ removed: [] }>()
const project = defineModel<ModProject | undefined>({ required: true })
const mode = shallowRef('archive')
const saving = shallowRef(false)
const error = shallowRef('')
const projects = useProjectsStore()
const session = useSessionStore()
const route = useRoute()
const router = useRouter()
const busy = computed(() => !!project.value && session.isProjectBusy(project.value.modId))
watch(project, (value, previous) => {
  if (value?.modId !== previous?.modId || value?.archived) {
    mode.value = value?.archived ? 'delete' : 'archive'
  }
  error.value = ''
})
async function reload(): Promise<void> {
  const id = project.value?.modId
  if (!id || saving.value) {
    return
  }
  saving.value = true
  try {
    project.value = await modService.get(id)
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { saving.value = false }
}
async function submit(): Promise<void> {
  const selected = project.value
  if (!selected || saving.value || busy.value || !['archive', 'delete'].includes(mode.value)) {
    return
  }
  saving.value = true
  error.value = ''
  try {
    if (mode.value === 'delete') {
      const result = await modService.remove(selected.modId, selected.revision)
      if (!result.removed) {
        throw new Error(result.message || '项目尚未移除')
      }
      projects.notice = result.message || '项目数据已永久删除'
    }
    else {
      await modService.archive(selected.modId, selected.revision, true)
      projects.notice = '项目已归档，可在“已归档项目”中恢复'
    }
    projects.removeLocal(selected.modId)
    session.forgetProject(selected.modId)
    project.value = undefined
    emit('removed')
    if (route.params.modId === selected.modId) {
      await router.push('/mods')
    }
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { saving.value = false }
}
</script>

<template>
  <Dialog :open="!!project" @update:open="value => !value && !saving && (project = undefined)">
    <DialogContent @interact-outside="saving && $event.preventDefault()" @escape-key-down="saving && $event.preventDefault()">
      <DialogHeader>
        <DialogTitle>移除项目：{{ project?.name }}</DialogTitle>
        <DialogDescription>选择保留数据以便恢复，或永久删除本地项目数据。</DialogDescription>
      </DialogHeader>
      <FieldGroup>
        <Field>
          <FieldLabel>移除方式</FieldLabel>
          <ToggleGroup v-model="mode" type="single" variant="outline" :disabled="saving || busy" class="flex-col items-stretch">
            <ToggleGroupItem v-if="!project?.archived" value="archive" class="h-auto justify-start whitespace-normal py-3">
              从侧边栏移除（归档，可恢复）
            </ToggleGroupItem>
            <ToggleGroupItem value="delete" class="h-auto justify-start whitespace-normal py-3">
              永久删除项目数据
            </ToggleGroupItem>
          </ToggleGroup>
        </Field>
        <p class="text-sm text-muted-foreground">
          {{ mode === 'delete' ? '永久删除本地源码、交付包、聊天、日志、项目记忆与本地安装记录，无法恢复。已安装到游戏的 MOD 和跨项目长期记忆保留。' : '保留源码、交付版本、聊天与项目记忆，恢复后继续使用原项目。' }}
        </p>
        <p v-if="busy" role="status" class="text-sm text-muted-foreground">
          项目正在执行任务或操作，请结束后重试。
        </p>
        <p v-if="error" role="alert" class="text-sm text-destructive">
          {{ error }}
        </p>
        <Button v-if="error" variant="outline" :disabled="saving" @click="reload">
          重新读取项目
        </Button>
      </FieldGroup>
      <DialogFooter>
        <Button variant="ghost" :disabled="saving" @click="project = undefined">
          取消
        </Button>
        <Button :variant="mode === 'delete' ? 'destructive' : 'default'" :disabled="saving || busy || !mode" @click="submit">
          {{ saving ? '处理中…' : mode === 'delete' ? '永久删除' : '归档项目' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
