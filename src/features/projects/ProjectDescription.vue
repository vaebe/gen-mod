<script setup lang="ts">
import type { ModProject } from '@/contracts/mod'
import { shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'
import { modService } from '@/features/projects/service'
import { useProjectsStore } from '@/stores/projects'

const props = defineProps<{ project: ModProject }>()
const projects = useProjectsStore()
const open = shallowRef(false)
const editing = shallowRef(false)
const description = shallowRef('')
const expected = shallowRef('')
const busy = shallowRef(false)
const error = shallowRef('')
watch(open, (value) => {
  if (value) {
    description.value = props.project.description
    expected.value = props.project.description
    editing.value = false
    error.value = ''
  }
})
async function save(): Promise<void> {
  if (busy.value || props.project.archived) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    const updated = await modService.describe(props.project.modId, description.value, expected.value)
    projects.upsert(updated)
    expected.value = updated.description
    description.value = updated.description
    editing.value = false
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
function cancelEdit(): void {
  description.value = expected.value
  error.value = ''
  editing.value = false
}
</script>

<template>
  <div class="shrink-0 border-b px-4 py-2 pr-14">
    <Button variant="ghost" class="max-w-full justify-start" :title="project.description || '查看项目描述'" @click="open = true">
      <span class="truncate">{{ project.name }}</span><span class="text-muted-foreground">项目描述</span>
    </Button>
  </div>
  <Dialog v-model:open="open">
    <DialogContent @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()">
      <DialogHeader>
        <DialogTitle>{{ project.name }}</DialogTitle>
        <DialogDescription>记录这个项目的用途，方便日后辨认。</DialogDescription>
      </DialogHeader>
      <form v-if="editing" class="flex flex-col gap-4" @submit.prevent="save">
        <FieldGroup>
          <Field :data-invalid="!!error">
            <FieldLabel for="project-description">
              项目描述（选填）
            </FieldLabel>
            <Textarea id="project-description" v-model="description" rows="6" :disabled="busy" :aria-invalid="!!error" />
            <FieldError v-if="error">
              {{ error }}
            </FieldError>
          </Field>
        </FieldGroup>
        <div class="flex justify-end gap-2">
          <Button type="button" variant="outline" :disabled="busy" @click="cancelEdit">
            取消
          </Button>
          <Button type="submit" :disabled="busy">
            {{ busy ? '正在保存…' : '保存' }}
          </Button>
        </div>
      </form>
      <template v-else>
        <p class="max-h-80 overflow-auto whitespace-pre-wrap break-words text-sm">
          {{ description || '暂无描述' }}
        </p>
        <Button v-if="!project.archived" variant="outline" class="justify-self-end" @click="editing = true">
          编辑描述
        </Button>
      </template>
    </DialogContent>
  </Dialog>
</template>
