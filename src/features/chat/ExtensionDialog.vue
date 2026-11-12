<script setup lang="ts">
import type { ExtensionRequest, ExtensionResponse } from '@/services/pi/types'
import { computed, onBeforeUnmount, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { useSessionStore } from '@/features/sessions/useSessionStore'

const session = useSessionStore()
const value = shallowRef('')
const error = shallowRef('')
const open = computed(() => !!session.extensionRequest && ['select', 'confirm', 'input'].includes(String(session.extensionRequest.method)))
const options = computed(() => Array.isArray(session.extensionRequest?.options) ? session.extensionRequest.options as string[] : [])
let timeout: ReturnType<typeof setTimeout> | undefined
async function reply(fields: ExtensionResponse, request: Readonly<ExtensionRequest> | undefined = session.extensionRequest): Promise<void> {
  if (!request) {
    return
  }
  if (timeout) {
    clearTimeout(timeout)
    timeout = undefined
  }
  try {
    await session.respondExtensionRequest(request, fields)
  }
  catch (cause) {
    error.value = (cause as Error).message
  }
}
watch(() => session.extensionRequest, (request) => {
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
      void reply({ cancelled: true }, request)
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
      <DialogHeader><DialogTitle>{{ session.extensionRequest?.title || '需要你的选择' }}</DialogTitle><DialogDescription>{{ session.extensionRequest?.message || '请完成此项输入以继续。' }}</DialogDescription></DialogHeader>
      <div v-if="session.extensionRequest?.method === 'select'" class="flex max-h-72 flex-col gap-2 overflow-auto">
        <Button v-for="option in options" :key="option" variant="outline" @click="reply({ value: option })">
          {{ option }}
        </Button>
      </div>
      <form v-else-if="session.extensionRequest?.method === 'input'" @submit.prevent="reply({ value })">
        <FieldGroup>
          <Field>
            <FieldLabel for="extension-input" class="sr-only">
              {{ session.extensionRequest?.title || '输入内容' }}
            </FieldLabel>
            <Input id="extension-input" v-model="value" :placeholder="String(session.extensionRequest.placeholder || '')" autofocus />
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
