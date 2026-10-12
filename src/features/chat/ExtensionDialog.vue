<script setup lang="ts">
import { computed, onBeforeUnmount, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { nativeCall } from '@/services/native'
import { useSessionStore } from '@/stores/session'

const session = useSessionStore()
const value = shallowRef('')
const error = shallowRef('')
const open = computed(() => !!session.uiRequest && ['select', 'confirm', 'input'].includes(String(session.uiRequest.method)))
const options = computed(() => Array.isArray(session.uiRequest?.options) ? session.uiRequest.options as string[] : [])
const answered = new Set<string>()
let timeout: ReturnType<typeof setTimeout> | undefined
async function reply(fields: Record<string, unknown>): Promise<void> {
  const request = session.uiRequest
  if (!request?.id || answered.has(request.id)) {
    return
  }
  answered.add(request.id)
  if (timeout) {
    clearTimeout(timeout)
    timeout = undefined
  }
  session.uiRequest = undefined
  try {
    // UI response 不期待普通 RPC response，不能占据请求 pending 队列。
    await nativeCall('send_pi_command', { generation: session.client.generation, command: { type: 'extension_ui_response', id: request.id, ...fields } })
  }
  catch (cause) {
    error.value = (cause as Error).message
    session.error = error.value
  }
}
watch(() => session.uiRequest, (request) => {
  if (timeout) {
    clearTimeout(timeout)
    timeout = undefined
  }
  value.value = ''
  if (!request || !open.value) {
    return
  }
  if (typeof request.timeout === 'number') {
    timeout = setTimeout(() => {
      void reply({ cancelled: true })
    }, request.timeout)
  }
})
onBeforeUnmount(() => {
  if (timeout) {
    clearTimeout(timeout)
  }
})
</script>

<template>
  <Dialog :open="open" @update:open="!$event && reply({ cancelled: true })">
    <DialogContent>
      <DialogHeader><DialogTitle>{{ session.uiRequest?.title || '需要你的选择' }}</DialogTitle><DialogDescription>{{ session.uiRequest?.message || '请完成此项输入以继续。' }}</DialogDescription></DialogHeader>
      <div v-if="session.uiRequest?.method === 'select'" class="flex max-h-72 flex-col gap-2 overflow-auto">
        <Button v-for="option in options" :key="option" variant="outline" @click="reply({ value: option })">
          {{ option }}
        </Button>
      </div>
      <form v-else-if="session.uiRequest?.method === 'input'" @submit.prevent="reply({ value })">
        <FieldGroup>
          <Field>
            <FieldLabel for="extension-input" class="sr-only">
              {{ session.uiRequest?.title || '输入内容' }}
            </FieldLabel>
            <Input id="extension-input" v-model="value" :placeholder="String(session.uiRequest.placeholder || '')" autofocus />
          </Field><Button type="submit" class="self-start">
            确认
          </Button>
        </FieldGroup>
      </form>
      <div v-else class="flex justify-end gap-3">
        <Button variant="outline" @click="reply({ confirmed: false })">
          否
        </Button><Button @click="reply({ confirmed: true })">
          是
        </Button>
      </div>
      <FieldError v-if="error">
        {{ error }}
      </FieldError>
    </DialogContent>
  </Dialog>
</template>
