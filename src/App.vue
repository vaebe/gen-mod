<script setup lang="ts">
import { defineAsyncComponent } from 'vue'

const WorkbenchLayout = defineAsyncComponent(() => import('@/features/workbench/WorkbenchLayout.vue'))
</script>

<template>
  <WorkbenchLayout />
</template>
