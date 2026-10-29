<script setup lang="ts">
import { Check, X } from '@lucide/vue'
import { onMounted, useTemplateRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Field, FieldError } from '@/components/ui/field'
import { Input } from '@/components/ui/input'

const props = defineProps<{ directory: boolean, disabled: boolean, pending: boolean, error: string }>()
const emit = defineEmits<{ submit: [name: string], cancel: [] }>()
const name = defineModel<string>('name', { required: true })
const field = useTemplateRef('field')
onMounted(() => {
  const input = field.value?.querySelector('input')
  input?.focus()
  const dot = props.directory ? -1 : name.value.lastIndexOf('.')
  input?.setSelectionRange(0, dot > 0 ? dot : name.value.length)
})
function keydown(event: KeyboardEvent): void {
  if (event.isComposing || event.keyCode === 229 || props.pending) {
    return
  }
  if (event.key === 'Enter' && !props.disabled) {
    event.preventDefault()
    emit('submit', name.value)
  }
  else if (event.key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    emit('cancel')
  }
}
</script>

<template>
  <div ref="field" class="min-w-0 flex-1 py-1 pr-2" @keydown="keydown">
    <Field :data-invalid="!!error" class="gap-1">
      <div class="flex items-center gap-1">
        <Input v-model="name" :aria-label="directory ? '文件夹名称' : '文件名称'" :aria-invalid="!!error" :disabled="disabled" class="h-7 min-w-0 px-1.5" />
        <Button variant="ghost" size="icon-sm" :disabled="disabled || !name.trim()" aria-label="确认名称" @click="emit('submit', name)">
          <Check />
        </Button>
        <Button variant="ghost" size="icon-sm" :disabled="pending" aria-label="取消改名或新建" @click="emit('cancel')">
          <X />
        </Button>
      </div>
      <FieldError v-if="error" :errors="[{ message: error }]" />
    </Field>
  </div>
</template>
