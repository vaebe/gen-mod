<script setup lang="ts">
import type { ModProject } from '@/contracts/mod'
import { ArrowUpRight, Folder, Plus } from '@lucide/vue'
import { defineAsyncComponent, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '@/components/ui/empty'
import { useProjectsStore } from '@/stores/projects'
import NewModDialog from './NewModDialog.vue'

const projects = useProjectsStore()
const createOpen = shallowRef(false)
const archivedOpen = shallowRef(false)
const removing = shallowRef<ModProject>()
const RemoveProjectDialog = defineAsyncComponent(() => import('./RemoveProjectDialog.vue'))
const ArchivedProjectsDialog = defineAsyncComponent(() => import('./ArchivedProjectsDialog.vue'))
</script>

<template>
  <section class="h-full overflow-auto px-6 py-10 lg:px-12">
    <div class="mx-auto max-w-4xl">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="mb-3 text-xs tracking-[0.15em] text-muted-foreground">
            你的创作空间
          </p><h1 class="text-2xl font-semibold tracking-tight">
            MOD 项目
          </h1><p class="mt-2 text-sm text-muted-foreground">
            从一个想法开始，保留每次修改与交付。
          </p>
        </div><div class="flex flex-wrap gap-2">
          <Button variant="outline" @click="archivedOpen = true">
            已归档项目
          </Button><Button @click="createOpen = true">
            <Plus class="size-4" />新建 MOD
          </Button>
        </div>
      </div>
      <p v-if="projects.notice" role="status" class="mt-4 text-sm text-muted-foreground">
        {{ projects.notice }}
      </p>
      <p v-if="projects.error" role="alert" class="mt-8 text-sm text-destructive">
        {{ projects.error }}<Button variant="outline" size="sm" @click="projects.refresh">
          重试
        </Button>
      </p>
      <p v-else-if="projects.loading" class="mt-8 text-sm text-muted-foreground">
        正在读取本地项目…
      </p>
      <Empty v-else-if="!projects.projects.length" class="mt-10 border">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <Folder />
          </EmptyMedia><EmptyTitle>
            第一个 MOD，从这里开始
          </EmptyTitle><EmptyDescription>
            描述想改变的游戏体验，选择 skill，让 AI 协助完成源码和交付。
          </EmptyDescription>
        </EmptyHeader><EmptyContent>
          <Button variant="outline" @click="createOpen = true">
            创建项目
          </Button>
        </EmptyContent>
      </Empty>
      <div v-else class="mt-8 grid gap-3 md:grid-cols-2">
        <Card v-for="project in projects.projects" :key="project.modId" class="flex h-full flex-col">
          <RouterLink :to="`/mods/${project.modId}/chat`" class="flex flex-1 flex-col gap-6 rounded-xl focus-visible:outline focus-visible:outline-ring">
            <CardHeader>
              <div class="flex items-center gap-3">
                <Folder class="size-4 text-muted-foreground" /><CardTitle class="truncate">
                  {{ project.name }}
                </CardTitle><ArrowUpRight class="ml-auto size-4 text-muted-foreground" />
              </div>
            </CardHeader><CardContent class="flex-1">
              <CardDescription class="line-clamp-2 min-h-10">
                {{ project.draftRequirement || '打开项目，继续描述你的需求。' }}
              </CardDescription>
            </CardContent>
          </RouterLink><CardFooter class="justify-between gap-3 text-xs text-muted-foreground">
            <span>{{ project.gameSkillName }} · {{ project.versions.length }} 个交付版本</span><Button variant="ghost" size="sm" @click="removing = project">
              移除
            </Button>
          </CardFooter>
        </Card>
      </div>
    </div><NewModDialog v-model:open="createOpen" />
    <RemoveProjectDialog v-model="removing" />
    <ArchivedProjectsDialog v-model:open="archivedOpen" />
  </section>
</template>
