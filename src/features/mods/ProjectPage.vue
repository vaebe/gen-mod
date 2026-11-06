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
  <section class="studio-page">
    <div class="studio-content">
      <header class="mb-7 flex flex-wrap items-center justify-between gap-5">
        <div>
          <h1 class="studio-title">
            MOD 项目
          </h1><p class="mt-2 text-sm leading-6 text-muted-foreground">
            从一个想法开始，保留每次修改与交付。
          </p>
        </div><div class="flex flex-wrap gap-2">
          <Button variant="outline" @click="archivedOpen = true">
            已归档项目
          </Button><Button @click="createOpen = true">
            <Plus class="size-4" />新建 MOD
          </Button>
        </div>
      </header>
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
      <Empty v-else-if="!projects.projects.length" class="mt-8 min-h-80 rounded-lg border border-dashed">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <Folder />
          </EmptyMedia><EmptyTitle>
            第一个 MOD，从这里开始
          </EmptyTitle><EmptyDescription>
            选择游戏，描述希望改变的体验，让 AI 协助制作 MOD。
          </EmptyDescription>
        </EmptyHeader><EmptyContent>
          <Button @click="createOpen = true">
            创建项目
          </Button>
        </EmptyContent>
      </Empty>
      <div v-else class="grid gap-4 lg:grid-cols-2">
        <Card v-for="project in projects.projects" :key="project.modId" class="studio-project-card flex h-full flex-col rounded-lg">
          <RouterLink :to="`/mods/${project.modId}/chat`" class="group flex flex-1 flex-col gap-4 rounded-lg pt-5 focus-visible:outline focus-visible:outline-ring">
            <CardHeader class="px-5 pt-0 pb-0">
              <div class="flex items-center gap-3">
                <Folder class="size-4 shrink-0 text-muted-foreground" /><CardTitle class="min-w-0 truncate text-base">
                  {{ project.name }}
                </CardTitle><ArrowUpRight class="ml-auto size-4 shrink-0 text-muted-foreground transition-colors group-hover:text-primary" />
              </div>
            </CardHeader><CardContent class="flex-1 px-5 pb-5">
              <CardDescription class="line-clamp-2 min-h-12 text-sm leading-6">
                {{ project.draftRequirement || '打开项目，继续描述你的需求。' }}
              </CardDescription>
            </CardContent>
          </RouterLink><CardFooter class="justify-between gap-3 border-t bg-background/25 px-5 py-3 text-xs text-muted-foreground">
            <span class="min-w-0"><span class="block truncate" :title="project.gameSkillName">{{ project.gameSkillName }}</span><span class="mt-1 block">{{ project.versions.length }} 个交付版本</span></span><Button variant="ghost" size="sm" class="text-muted-foreground" @click="removing = project">
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
