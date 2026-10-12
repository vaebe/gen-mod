<script setup lang="ts">
import type { ImageInput } from '@/contracts/session'
import { computed } from 'vue'
import { Dialog, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from '@/components/ui/dialog'

const props = defineProps<{ image: ImageInput, alt: string, compact?: boolean }>()
const source = computed(() => `data:${props.image.mimeType};base64,${props.image.data}`)
</script>

<template>
  <Dialog>
    <DialogTrigger as-child>
      <button type="button" :aria-label="`放大查看${alt}`" title="点击放大" class="shrink-0 cursor-zoom-in focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2" :class="compact ? 'rounded' : 'rounded-xl'">
        <img :src="source" :alt="alt" class="border bg-muted object-cover" :class="compact ? 'size-10 rounded' : 'size-20 rounded-xl'" loading="lazy">
      </button>
    </DialogTrigger>
    <DialogContent class="w-[95vw] max-w-[95vw] sm:max-w-[95vw]">
      <DialogTitle class="pr-6">
        图片预览
      </DialogTitle>
      <DialogDescription class="sr-only">
        {{ alt }}，按 Esc 或点击关闭按钮退出预览。
      </DialogDescription>
      <img :src="source" :alt="alt" class="max-h-[calc(90vh-6rem)] w-full object-contain">
    </DialogContent>
  </Dialog>
</template>
