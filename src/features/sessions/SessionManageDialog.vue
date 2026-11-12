<script setup lang="ts">
import { shallowRef, watch } from 'vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { sessionService } from '@/features/sessions/service'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import { useProjectsStore } from '@/stores/projects'

export interface SessionAction { modId: string, sessionId: string, name: string, kind: 'rename' | 'delete' }
const action = defineModel<SessionAction | undefined>({ required: true })
const name = shallowRef('')
const saving = shallowRef(false)
const error = shallowRef('')
const projects = useProjectsStore()
const session = useSessionStore()
const router = useRouter()
watch(action, (value) => {
  name.value = value?.name ?? ''
  error.value = ''
})
/** 删除当前会话前关闭其空闲进程；不清理源码、交付版本和任务日志。 */
async function submit(): Promise<void> {
  const selected = action.value
  if (!selected || saving.value || (selected.kind === 'delete' && session.isBusy(selected.modId, selected.sessionId))) {
    return
  }
  saving.value = true
  error.value = ''
  const active = session.isSelected(selected)
  try {
    if (selected.kind === 'delete' && active) {
      await session.close()
    }
    const project = await (selected.kind === 'delete'
      ? sessionService.remove({ modId: selected.modId, sessionId: selected.sessionId })
      : sessionService.rename({ modId: selected.modId, sessionId: selected.sessionId, name: name.value }))
    projects.upsert(project)
    action.value = undefined
    if (selected.kind === 'delete' && active) {
      const next = project.sessions[project.sessions.length - 1]
      await router.push(next ? { path: `/mods/${project.modId}/chat`, query: { session: next.sessionId } } : '/mods')
    }
  }
  catch (cause) {
    error.value = (cause as Error).message
    await projects.refresh()
  }
  finally { saving.value = false }
}
</script>

<template>
  <Dialog :open="!!action" @update:open="value => !value && !saving && (action = undefined)">
    <DialogContent @interact-outside="saving && $event.preventDefault()" @escape-key-down="saving && $event.preventDefault()">
      <DialogHeader><DialogTitle>{{ action?.kind === 'delete' ? '删除会话' : '重命名会话' }}</DialogTitle><DialogDescription>{{ action?.kind === 'delete' ? '删除会话及对话历史，无法撤销。项目源码、已生成版本和任务日志会保留。' : '修改左侧菜单中显示的会话名称。' }}</DialogDescription></DialogHeader>
      <form @submit.prevent="submit">
        <FieldGroup>
          <p v-if="action?.kind === 'delete'" class="break-words text-sm">
            {{ action.name }}
          </p>
          <Field v-else :data-disabled="saving">
            <FieldLabel for="session-name">
              会话名称
            </FieldLabel>
            <Input id="session-name" v-model="name" maxlength="120" required autofocus :disabled="saving" />
          </Field>
          <FieldError v-if="error">
            {{ error }}
          </FieldError>
          <div class="flex justify-end gap-2">
            <Button type="button" variant="ghost" :disabled="saving" @click="action = undefined">
              取消
            </Button><Button type="submit" :variant="action?.kind === 'delete' ? 'destructive' : 'default'" :disabled="saving || (action?.kind === 'delete' && session.isBusy(action.modId, action.sessionId)) || (action?.kind === 'rename' && !name.trim())">
              {{ saving ? '处理中…' : action?.kind === 'delete' ? '删除会话' : '保存' }}
            </Button>
          </div>
        </FieldGroup>
      </form>
    </DialogContent>
  </Dialog>
</template>
