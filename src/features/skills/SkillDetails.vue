<script setup lang="ts">
import type { SkillView } from '@/contracts/skill'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'

defineProps<{ skill: SkillView, disabled: boolean }>()
defineEmits<{ toggle: [enabled: boolean], select: [] }>()
</script>

<template>
  <li class="flex items-center gap-4 py-1">
    <Button type="button" variant="ghost" class="h-auto min-w-0 flex-1 flex-col items-stretch whitespace-normal px-3 py-4 text-left" :aria-label="`查看 ${skill.name} 的 SKILL.md`" @click="$emit('select')">
      <span class="block break-words text-sm font-medium">{{ skill.name }}</span>
      <span class="mt-1 block break-words text-sm leading-6 text-muted-foreground">{{ skill.description }}</span>
    </Button>
    <Switch :model-value="skill.enabled" :disabled="disabled" :aria-label="`启用 ${skill.name}`" class="mr-3" @update:model-value="$emit('toggle', $event)" />
  </li>
</template>
