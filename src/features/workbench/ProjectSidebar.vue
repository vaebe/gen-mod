<script setup lang="ts">
import type { SessionAction } from './SessionManageDialog.vue'
import type { ModProject } from '@/contracts/mod'
import { Folder, MoreVertical, Pencil, Settings, Sparkles, Trash2 } from '@lucide/vue'
import { defineAsyncComponent, shallowRef } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Sidebar, SidebarContent, SidebarFooter, SidebarGroup, SidebarGroupLabel, SidebarMenu, SidebarMenuAction, SidebarMenuButton, SidebarMenuItem, SidebarMenuSub, SidebarMenuSubButton, SidebarMenuSubItem } from '@/components/ui/sidebar'
import { useProjectsStore } from '@/stores/projects'
import { useSessionStore } from '@/stores/session'
import SessionManageDialog from './SessionManageDialog.vue'
import SessionRunIndicator from './SessionRunIndicator.vue'

const projects = useProjectsStore()
const GameIcon = defineAsyncComponent(() => import('../mods/GameIcon.vue'))
const route = useRoute()
const router = useRouter()
const session = useSessionStore()
const action = shallowRef<SessionAction>()
const removing = shallowRef<ModProject>()
const RemoveProjectDialog = defineAsyncComponent(() => import('../mods/RemoveProjectDialog.vue'))
const collapsedProjects = shallowRef(new Set<string>())
/** 分组展开状态只影响列表展示，不切换或停止当前会话。 */
function toggleProject(modId: string): void {
  const next = new Set(collapsedProjects.value)
  if (next.has(modId)) {
    next.delete(modId)
  }
  else { next.add(modId) }
  collapsedProjects.value = next
}
/** 路由请求由 ChatPage 保存草稿并创建会话，左侧不重复建立执行连接。 */
async function createSession(modId: string): Promise<void> {
  const next = new Set(collapsedProjects.value)
  next.delete(modId)
  collapsedProjects.value = next
  try {
    await router.push({ path: `/mods/${modId}/chat`, query: { new: crypto.randomUUID() } })
  }
  catch (cause) { session.error = (cause as Error).message }
}
</script>

<template>
  <Sidebar collapsible="none" class="shrink-0 border-r border-sidebar-border">
    <SidebarContent class="gap-2 px-2 pt-3">
      <SidebarGroup class="pb-1">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton as-child class="h-9" :is-active="route.path === '/mods'">
              <RouterLink to="/mods">
                <Folder />全部项目
              </RouterLink>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroup>
      <SidebarGroup class="border-t border-sidebar-border pt-4">
        <SidebarGroupLabel class="mb-2 h-6 text-xs">
          最近的项目
        </SidebarGroupLabel>
        <p v-if="projects.loading" class="px-2 py-3 text-xs text-muted-foreground">
          正在读取项目…
        </p>
        <p v-else-if="projects.error" role="alert" class="px-2 py-3 text-xs text-destructive">
          {{ projects.error }}
        </p>
        <p v-else-if="!projects.projects.length" class="px-2 py-3 text-xs text-muted-foreground">
          创建第一个 MOD 后显示在这里
        </p>
        <SidebarMenu>
          <SidebarMenuItem v-for="project in projects.projects" :key="project.modId">
            <SidebarMenuButton :is-active="route.params.modId === project.modId" :aria-expanded="!collapsedProjects.has(project.modId)" :aria-controls="`project-sessions-${project.modId}`" :title="`${collapsedProjects.has(project.modId) ? '展开' : '收起'}${project.name}`" @click="toggleProject(project.modId)">
              <GameIcon :game-id="project.gameId" :expanded="!collapsedProjects.has(project.modId)" /><span>{{ project.name }}</span>
            </SidebarMenuButton>
            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <SidebarMenuAction :aria-label="`${project.name}的操作`" title="项目操作">
                  <MoreVertical class="size-4" />
                </SidebarMenuAction>
              </DropdownMenuTrigger>
              <DropdownMenuContent>
                <DropdownMenuGroup>
                  <DropdownMenuItem @select="createSession(project.modId)">
                    新增会话
                  </DropdownMenuItem><DropdownMenuItem @select="removing = project">
                    移除项目…
                  </DropdownMenuItem>
                </DropdownMenuGroup>
              </DropdownMenuContent>
            </DropdownMenu>
            <SidebarMenuSub v-show="project.sessions.length && !collapsedProjects.has(project.modId)" :id="`project-sessions-${project.modId}`" class="mx-0 w-full translate-x-0 border-l-0 px-0">
              <SidebarMenuSubItem v-for="item in project.sessions" :key="item.sessionId" class="group/session relative">
                <!-- 右侧操作区固定占位，标题只按实际剩余宽度截断。 -->
                <SidebarMenuSubButton as-child :is-active="route.params.modId === project.modId && session.sessionId === item.sessionId" class="w-full translate-x-0 pr-14 pl-3">
                  <RouterLink :to="{ path: `/mods/${project.modId}/chat`, query: { session: item.sessionId } }" :title="`${item.name || '新会话'}${item.readOnly ? ' · 只读' : ''}`">
                    <span class="min-w-0 flex-1 truncate">{{ item.name || '新会话' }}{{ item.readOnly ? ' · 只读' : '' }}</span>
                  </RouterLink>
                </SidebarMenuSubButton>
                <div class="absolute top-1/2 right-1 flex -translate-y-1/2 items-center gap-1">
                  <span class="flex size-4 items-center [&>span]:mr-0"><SessionRunIndicator :busy="session.isRunning(project.modId, item.sessionId)" :label="session.stateFor(project.modId, item.sessionId)?.activity" /></span>
                  <DropdownMenu>
                    <DropdownMenuTrigger as-child>
                      <Button type="button" variant="ghost" size="icon-xs" :aria-label="`${item.name || '新会话'}的操作`" title="会话操作" class="opacity-0 group-hover/session:opacity-100 group-focus-within/session:opacity-100 data-[state=open]:opacity-100">
                        <MoreVertical class="size-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start" :side-offset="4" class="min-w-40">
                      <DropdownMenuGroup>
                        <DropdownMenuItem @select="action = { modId: project.modId, sessionId: item.sessionId, name: item.name || '新会话', kind: 'rename' }">
                          <Pencil class="size-4" />重命名
                        </DropdownMenuItem><DropdownMenuItem class="text-destructive" :disabled="session.isBusy(project.modId, item.sessionId)" @select="action = { modId: project.modId, sessionId: item.sessionId, name: item.name || '新会话', kind: 'delete' }">
                          <Trash2 class="size-4" />删除会话
                        </DropdownMenuItem>
                      </DropdownMenuGroup>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </SidebarMenuSubItem>
            </SidebarMenuSub>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroup>
    </SidebarContent>
    <SidebarFooter class="mx-4 border-t border-sidebar-border px-0 pt-3 pb-4">
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton as-child :is-active="route.path === '/skills'">
            <RouterLink to="/skills">
              <Sparkles />Skills
            </RouterLink>
          </SidebarMenuButton>
        </SidebarMenuItem>
        <SidebarMenuItem>
          <SidebarMenuButton as-child :is-active="route.path === '/settings'">
            <RouterLink to="/settings">
              <Settings />设置
            </RouterLink>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarFooter>
  </Sidebar>
  <SessionManageDialog v-model="action" />
  <RemoveProjectDialog v-model="removing" />
</template>
