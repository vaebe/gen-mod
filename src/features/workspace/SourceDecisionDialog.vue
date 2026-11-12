<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'

defineProps<{ question?: { title: string, detail: string, choices: { value: string, label: string }[] } }>()
const emit = defineEmits<{ answer: [value: string] }>()
</script>

<template>
  <Dialog :open="!!question" @update:open="!$event && emit('answer', 'cancel')">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>{{ question?.title }}</DialogTitle>
        <DialogDescription class="whitespace-pre-wrap break-all">
          {{ question?.detail }}
        </DialogDescription>
      </DialogHeader>
      <div class="flex flex-wrap justify-end gap-2">
        <Button v-for="choice in question?.choices" :key="choice.value" variant="outline" @click="emit('answer', choice.value)">
          {{ choice.label }}
        </Button>
        <Button variant="ghost" @click="emit('answer', 'cancel')">
          取消
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>
