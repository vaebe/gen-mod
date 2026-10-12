<script setup lang="ts">
import { Spinner } from '@/components/ui/spinner'

defineProps<{ busy: boolean, label?: string }>()
</script>

<template>
  <span v-if="busy" class="mr-5 flex shrink-0 items-center text-muted-foreground" role="status" :aria-label="label || '会话运行中'" :title="label || '会话运行中'">
    <Spinner class="size-3.5 motion-reduce:animate-none" aria-hidden="true" />
  </span>
</template>
