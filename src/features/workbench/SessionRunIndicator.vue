<script setup lang="ts">
defineProps<{ busy: boolean; label?: string }>();
</script>
<template>
  <span v-if="busy" class="mr-5 flex shrink-0 items-center text-muted-foreground" role="status" :aria-label="label || '会话运行中'" :title="label || '会话运行中'">
    <svg viewBox="0 0 20 20" class="size-3.5 animate-spin motion-reduce:animate-none" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="10" cy="10" r="7" opacity=".2" /><path d="M10 3a7 7 0 0 1 7 7" stroke-linecap="round" /></svg>
  </span>
</template>
