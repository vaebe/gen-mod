<script setup lang="ts">
import { shallowRef } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Boxes, Folder, FolderOpen, Settings, Sparkles, MoreVertical, Pencil, Trash2 } from "@lucide/vue";
import { useProjectsStore } from "@/stores/projects";
import { Sidebar, SidebarContent, SidebarFooter, SidebarGroup, SidebarGroupLabel, SidebarHeader, SidebarMenu, SidebarMenuButton, SidebarMenuItem, SidebarMenuAction, SidebarMenuSub, SidebarMenuSubItem, SidebarMenuSubButton } from "@/components/ui/sidebar";
import { DropdownMenuRoot, DropdownMenuPortal, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "reka-ui";
import SessionRunIndicator from "./SessionRunIndicator.vue";
import SessionManageDialog, { type SessionAction } from "./SessionManageDialog.vue";
import { useSessionStore } from "@/stores/session";
const projects = useProjectsStore();
const route = useRoute();
const router = useRouter();
const session = useSessionStore();
const action = shallowRef<SessionAction>();
const collapsedProjects = shallowRef(new Set<string>());
/** 分组展开状态只影响列表展示，不切换或停止当前会话。 */
function toggleProject(modId: string): void {
  const next = new Set(collapsedProjects.value);
  if (next.has(modId)) { next.delete(modId); } else { next.add(modId); }
  collapsedProjects.value = next;
}
/** 路由请求由 ChatPage 保存草稿并创建会话，左侧不重复建立执行连接。 */
async function createSession(modId: string): Promise<void> {
  const next = new Set(collapsedProjects.value); next.delete(modId); collapsedProjects.value = next;
  try { await router.push({ path: `/mods/${modId}/chat`, query: { new: crypto.randomUUID() } }); }
  catch (cause) { session.error = (cause as Error).message; }
}
</script>

<template>
  <Sidebar>
    <SidebarHeader class="px-4 pt-5 pb-4">
      <div class="flex items-center text-sm font-semibold tracking-wide">
        <Boxes class="size-5" />GenMod
      </div>
      <p class="text-xs text-muted-foreground">把想法变成游戏里的变化</p>
    </SidebarHeader>
    <SidebarContent>
      <SidebarGroup><SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton as-child :is-active="route.path === '/mods'">
            <RouterLink to="/mods">
              <Folder />全部项目
            </RouterLink>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu></SidebarGroup>
      <SidebarGroup><SidebarGroupLabel>最近的项目</SidebarGroupLabel>
        <p v-if="projects.loading" class="px-2 py-3 text-xs text-muted-foreground">正在读取项目…</p>
        <p v-else-if="projects.error" role="alert" class="px-2 py-3 text-xs text-destructive">{{ projects.error }}</p>
        <p v-else-if="!projects.projects.length" class="px-2 py-3 text-xs text-muted-foreground">创建第一个 MOD 后显示在这里</p>
        <SidebarMenu>
          <SidebarMenuItem v-for="project in projects.projects" :key="project.modId">
            <SidebarMenuButton :is-active="route.params.modId === project.modId" :aria-expanded="!collapsedProjects.has(project.modId)" :aria-controls="`project-sessions-${project.modId}`" :title="`${collapsedProjects.has(project.modId) ? '展开' : '收起'}${project.name}`" @click="toggleProject(project.modId)"><Folder v-if="collapsedProjects.has(project.modId)" /><FolderOpen v-else /><span>{{ project.name }}</span></SidebarMenuButton>
            <SidebarMenuAction :aria-label="`为${project.name}新增会话`" title="新增会话" class="opacity-0 peer-hover/menu-button:opacity-100 peer-focus-visible/menu-button:opacity-100 hover:opacity-100 focus-visible:opacity-100" @click.stop="createSession(project.modId)"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 4H8a4 4 0 0 0-4 4v8a4 4 0 0 0 4 4h8a4 4 0 0 0 4-4v-2" /><path d="m15 4 5 5M10 14l4-1L21 6a2.12 2.12 0 0 0-3-3l-7 7-1 4Z" /></svg></SidebarMenuAction>
            <SidebarMenuSub v-show="project.sessions.length && !collapsedProjects.has(project.modId)" :id="`project-sessions-${project.modId}`" class="mx-0 w-full translate-x-0 border-l-0 px-0">
              <SidebarMenuSubItem v-for="item in project.sessions" :key="item.sessionId" class="group/session relative">
                <!-- 右侧操作区固定占位，标题只按实际剩余宽度截断。 -->
                <SidebarMenuSubButton as-child :is-active="route.params.modId === project.modId && session.sessionId === item.sessionId" class="w-full translate-x-0 pl-3" :class="session.isBusy(project.modId, item.sessionId) ? 'pr-14' : 'pr-9'">
                  <RouterLink :to="{ path: `/mods/${project.modId}/chat`, query: { session: item.sessionId } }" :title="`${item.name || '新会话'}${item.readOnly ? ' · 只读' : ''}`">
                    <span class="min-w-0 flex-1 truncate">{{ item.name || '新会话' }}{{ item.readOnly ? ' · 只读' : '' }}</span>
                  </RouterLink>
                </SidebarMenuSubButton>
                <div class="absolute top-1/2 right-1 flex -translate-y-1/2 items-center gap-1">
                  <span v-if="session.isBusy(project.modId, item.sessionId)" class="flex items-center [&>span]:mr-0"><SessionRunIndicator :busy="true" :label="session.stateFor(project.modId,item.sessionId)?.activity" /></span>
                <DropdownMenuRoot><DropdownMenuTrigger as-child><button type="button" :aria-label="`${item.name || '新会话'}的操作`" title="会话操作" class="flex size-6 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-colors hover:bg-sidebar-accent hover:text-sidebar-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring group-hover/session:opacity-100 group-focus-within/session:opacity-100 data-[state=open]:opacity-100"><MoreVertical class="size-4" /></button></DropdownMenuTrigger><DropdownMenuPortal><DropdownMenuContent align="start" :side-offset="4" class="z-50 min-w-32 rounded-md border bg-popover p-1 text-popover-foreground shadow-md"><DropdownMenuItem class="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none data-[highlighted]:bg-accent" @select="action = { modId: project.modId, sessionId: item.sessionId, name: item.name || '新会话', kind: 'rename' }"><Pencil class="size-4" />重命名</DropdownMenuItem><DropdownMenuItem :disabled="session.isBusy(project.modId,item.sessionId)" class="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-destructive outline-none data-[highlighted]:bg-accent" @select="action = { modId: project.modId, sessionId: item.sessionId, name: item.name || '新会话', kind: 'delete' }"><Trash2 class="size-4" />删除会话</DropdownMenuItem></DropdownMenuContent></DropdownMenuPortal></DropdownMenuRoot>
                </div>
              </SidebarMenuSubItem>
            </SidebarMenuSub>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroup>
    </SidebarContent>
    <SidebarFooter class="pb-4"><SidebarMenu>
      <SidebarMenuItem><SidebarMenuButton as-child :is-active="route.path === '/skills'"><RouterLink to="/skills"><Sparkles />Skills</RouterLink></SidebarMenuButton></SidebarMenuItem>
      <SidebarMenuItem><SidebarMenuButton as-child :is-active="route.path === '/settings'"><RouterLink to="/settings"><Settings />设置</RouterLink></SidebarMenuButton></SidebarMenuItem>
    </SidebarMenu></SidebarFooter>
  </Sidebar>
  <SessionManageDialog v-model="action" />
</template>
