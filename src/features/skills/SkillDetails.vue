<script setup lang="ts">
import type { SkillView } from "@/contracts/skill";
import { Button } from "@/components/ui/button";
defineProps<{ skill: SkillView; loaded: boolean; disabled: boolean }>();
defineEmits<{ toggle: [] }>();
</script>
<template>
  <article class="rounded-lg border p-4">
    <div class="flex items-start justify-between gap-3"><div><h2 class="font-medium">{{ skill.name }} <span class="text-xs text-muted-foreground">{{ skill.version }}</span></h2><p class="mt-2 text-sm text-muted-foreground">{{ skill.description }}</p></div><Button variant="outline" size="sm" :disabled="disabled" @click="$emit('toggle')">{{ skill.enabled ? '禁用' : '启用' }}</Button></div>
    <div class="mt-3 flex flex-wrap gap-3 text-xs text-muted-foreground"><span>{{ skill.category === 'game' ? '游戏' : '通用' }}</span><span>{{ skill.source === 'builtin' ? '内置 · 只读' : '本地导入' }}</span><span>已安装</span><span>{{ skill.enabled ? '已启用' : '已禁用' }}</span><span v-if="loaded">本会话已加载</span></div>
    <p v-if="skill.error" role="alert" class="mt-3 text-xs text-destructive">{{ skill.error }}</p>
    <p v-if="skill.dependencies.length" class="mt-3 text-xs">依赖：{{ skill.dependencies.join('、') }}</p>
    <details v-if="skill.scripts.length" class="mt-3 text-xs"><summary>包含 {{ skill.scripts.length }} 个脚本</summary><ul class="mt-2 space-y-1 font-mono"><li v-for="script in skill.scripts" :key="script">{{ script }}</li></ul></details>
  </article>
</template>
