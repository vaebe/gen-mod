<script setup lang="ts">
import { shallowRef } from "vue";
import { ArrowUpRight, Folder, Plus } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import NewModDialog from "./NewModDialog.vue";
import { useProjectsStore } from "@/stores/projects";
const projects = useProjectsStore();
const createOpen = shallowRef(false);
</script>
<template>
  <section class="h-full overflow-auto px-6 py-10 lg:px-12"><div class="mx-auto max-w-4xl">
    <div class="flex items-start justify-between gap-4"><div><p class="mb-3 text-xs tracking-[0.15em] text-muted-foreground">你的创作空间</p><h1 class="text-2xl font-semibold tracking-tight">MOD 项目</h1><p class="mt-2 text-sm text-muted-foreground">从一个想法开始，保留每次修改与交付。</p></div><Button @click="createOpen = true"><Plus class="size-4" />新建 MOD</Button></div>
    <p v-if="projects.error" role="alert" class="mt-8 text-sm text-destructive">{{ projects.error }}<Button variant="outline" size="sm" @click="projects.refresh">重试</Button></p>
    <p v-else-if="projects.loading" class="mt-8 text-sm text-muted-foreground">正在读取本地项目…</p>
    <div v-else-if="!projects.projects.length" class="mt-10 rounded-xl border border-dashed px-6 py-16 text-center"><Folder class="mx-auto mb-4 size-8 text-muted-foreground" /><h2 class="text-base font-medium">第一个 MOD，从这里开始</h2><p class="mx-auto mt-2 mb-6 max-w-sm text-sm leading-6 text-muted-foreground">描述想改变的游戏体验，选择 skill，让 AI 协助完成源码和交付。</p><Button variant="outline" @click="createOpen = true">创建项目</Button></div>
    <div v-else class="mt-8 grid gap-3 md:grid-cols-2"><RouterLink v-for="project in projects.projects" :key="project.modId" :to="`/mods/${project.modId}/chat`" class="group rounded-xl border bg-card p-5 transition-colors hover:bg-accent/40"><div class="flex items-center gap-3"><Folder class="size-4 text-muted-foreground" /><h2 class="truncate text-sm font-medium">{{ project.name }}</h2><ArrowUpRight class="ml-auto size-4 text-muted-foreground" /></div><p class="mt-3 line-clamp-2 min-h-10 text-xs leading-5 text-muted-foreground">{{ project.draftRequirement || '打开项目，继续描述你的需求。' }}</p><div class="mt-4 flex justify-between text-[11px] text-muted-foreground"><span>{{ project.gameSkillName }}</span><span>{{ project.versions.length }} 个交付版本</span></div></RouterLink></div>
  </div><NewModDialog v-model:open="createOpen" /></section>
</template>
