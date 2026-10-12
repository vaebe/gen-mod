<script setup lang="ts">
import type { SkillView } from '@/contracts/skill'
import { SwitchRoot, SwitchThumb } from 'reka-ui'

defineProps<{ skill: SkillView, disabled: boolean }>()
defineEmits<{ toggle: [enabled: boolean], select: [] }>()
</script>

<template>
  <li class="flex items-center gap-4 py-1">
    <button type="button" class="min-w-0 flex-1 rounded-md px-3 py-4 text-left transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" :aria-label="`查看 ${skill.name} 的 SKILL.md`" @click="$emit('select')">
      <span class="block break-words text-sm font-medium">{{ skill.name }}</span>
      <span class="mt-1 block break-words text-sm leading-6 text-muted-foreground">{{ skill.description }}</span>
    </button>
    <SwitchRoot :model-value="skill.enabled" :disabled="disabled" :aria-label="`启用 ${skill.name}`" class="mr-3 inline-flex h-6 w-11 shrink-0 items-center rounded-full border-2 border-transparent bg-input transition-colors data-[state=checked]:bg-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50" @update:model-value="$emit('toggle', $event)">
      <SwitchThumb class="pointer-events-none block size-5 rounded-full bg-background shadow-sm transition-transform data-[state=checked]:translate-x-5" />
    </SwitchRoot>
  </li>
</template>
