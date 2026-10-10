<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useIntervalFn } from "@vueuse/core";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSessionStore } from "@/stores/session";
import { nativeCall } from "@/services/native";
import { Button } from "@/components/ui/button";
import type { ModProject } from "@/contracts/mod";
import { useProjectsStore } from "@/stores/projects";
import type { FileChange, FileDiff as FileDiffData } from "@/contracts/changes";
import ChangedFiles from "./ChangedFiles.vue";
import FileDiff from "./FileDiff.vue";
import SourceBrowser from "./SourceBrowser.vue";
import ArtifactPanel from "@/features/mods/ArtifactPanel.vue";
const session = useSessionStore();
const changes = shallowRef<FileChange[]>([]);
const operations = shallowRef<{toolCallId: string;operation: string;path?: string;sourceRevision: number;fileChanges?: {path: string}[]}[]>([]);
const diff = shallowRef<FileDiffData>();
const selected = shallowRef("");
const error = shallowRef("");
const project = shallowRef<ModProject>();
const projects = useProjectsStore();
const artifactProject = computed(() => projects.projects.find(item => item.modId === session.modId) ?? (project.value?.modId === session.modId ? project.value : undefined));
const artifactVersionId = shallowRef("");
const artifactDetailsOpen = shallowRef(false);
const selectedArtifact = computed(() => artifactProject.value?.versions.find(version => version.versionId === artifactVersionId.value));
function viewArtifact(id: string): void {
  artifactVersionId.value = id;
  artifactDetailsOpen.value = true;
}
watch(() => session.modId, () => { artifactVersionId.value = ""; artifactDetailsOpen.value = false; });
// 与版本管理页一致，只标记目标当前确认已安装的版本。
const installedVersionIds = computed(() => new Set(artifactProject.value?.installedTargets.flatMap(target => target.state === "known" && target.versionId ? [target.versionId] : []) ?? []));
type RuntimeLogEntry = { timestamp: string; generation: number; taskId?: string; message: string };
const runtimeLogs = shallowRef<RuntimeLogEntry[]>([]);
const logError = shallowRef("");
const selectedTab = shallowRef("files");
const fileMode = shallowRef("all");
let readingLogs = false;
/** 只刷新可见日志页；切换会话后丢弃旧请求，历史从磁盘恢复。 */
async function refreshLogs(): Promise<void> {
  if (readingLogs || selectedTab.value !== "logs" || !session.modId || !session.sessionId) { return; }
  readingLogs = true;
  const modId = session.modId;
  const sessionId = session.sessionId;
  try {
    const entries = await nativeCall<RuntimeLogEntry[]>("read_session_runtime_logs", { modId, sessionId });
    if (modId === session.modId && sessionId === session.sessionId) { runtimeLogs.value = entries; logError.value = ""; }
  } catch (cause) { if (modId === session.modId && sessionId === session.sessionId) { logError.value = (cause as Error).message; } }
  finally { readingLogs = false; }
}
watch(() => [session.modId, session.sessionId], () => { runtimeLogs.value = []; logError.value = ""; void refreshLogs(); });
watch(selectedTab, () => { void refreshLogs(); });
useIntervalFn(() => { void refreshLogs(); }, 3000);
watch(() => [session.modId, session.artifactRevision, session.changeRevision], async (_value, _old, onCleanup) => {
  let valid = true; onCleanup(() => { valid = false; }); project.value = undefined;
  if (!session.modId) { return; }
  try { const result = await nativeCall<ModProject>("get_mod", { modId: session.modId }); if (valid) { project.value = result; projects.upsert(result); } }
  catch (cause) { if (valid) { error.value = (cause as Error).message; } }
}, { immediate: true });
async function openLogs(): Promise<void> {
  if (!session.task) { return; }
  try { await nativeCall("open_task_logs", { modId: session.modId, taskId: session.task.taskId }); }
  catch (cause) { error.value = (cause as Error).message; }
}
let generation = 0;
async function select(path: string): Promise<void> {
  const current = ++generation;
  diff.value = undefined;
  selected.value = path;
  error.value = "";
  if (!session.task) { return; }
  try {
    const result = await nativeCall<FileDiffData>("get_task_file_diff", { modId: session.modId, taskId: session.task.taskId, relativePath: path });
    if (current === generation) { diff.value = result; }
  } catch (cause) { if (current === generation) { error.value = (cause as Error).message; } }
}
watch(() => [session.task?.taskId, session.changeRevision], async (_value, _old, onCleanup) => {
  let valid = true;
  onCleanup(() => { valid = false; });
  changes.value = []; operations.value=[];
  diff.value = undefined;
  error.value = "";
  generation += 1;
  if (!session.task) { return; }
  try {
    const recorded = await nativeCall<typeof operations.value>("list_task_operations",{modId:session.modId,taskId:session.task.taskId});
    if (!valid) { return; }
    operations.value=recorded;
    const result = await nativeCall<FileChange[]>("list_task_changes", { modId: session.modId, taskId: session.task.taskId });
    if (!valid) { return; }
    changes.value = result;
    if (selected.value && result.some((item) => item.path === selected.value)) { await select(selected.value); }
  } catch (cause) { if (valid) { error.value = (cause as Error).message; } }
}, { immediate: true });
</script>

<template>
  <section class="flex h-full min-h-0 flex-col border-l bg-card/50">
    <div class="px-4 pt-4 pb-3 text-sm font-medium">项目上下文</div>
    <Tabs v-model="selectedTab" class="flex min-h-0 flex-1 flex-col px-3"><TabsList class="w-full"><TabsTrigger value="files" class="flex-1">文件</TabsTrigger><TabsTrigger value="logs" class="flex-1">日志</TabsTrigger><TabsTrigger value="artifacts" class="flex-1">产物</TabsTrigger></TabsList>
      <TabsContent value="files" class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <div class="flex shrink-0 gap-1 border-b p-2" role="group" aria-label="文件查看范围">
          <Button size="sm" :variant="fileMode === 'all' ? 'secondary' : 'ghost'" class="h-7 flex-1 text-xs" :aria-pressed="fileMode === 'all'" @click="fileMode = 'all'">全部文件</Button>
          <Button size="sm" :variant="fileMode === 'changes' ? 'secondary' : 'ghost'" class="h-7 flex-1 text-xs" :aria-pressed="fileMode === 'changes'" @click="fileMode = 'changes'">当前变更</Button>
        </div>
        <SourceBrowser v-show="fileMode === 'all'" :mod-id="session.modId" :revision="session.changeRevision" :active="selectedTab === 'files' && fileMode === 'all'" />
        <ScrollArea v-if="fileMode === 'changes'" class="min-h-0 flex-1"><div v-if="session.task" class="space-y-2 border-b p-3 text-xs"><p class="font-medium">本任务工具操作记录</p><p v-if="!operations.length" class="text-muted-foreground">暂无已记录的源码写入操作。</p><p v-for="item in operations" :key="item.toolCallId" class="break-all text-muted-foreground">{{ item.operation }} · {{ item.path || (item.fileChanges?.length + ' 个文件') }} · revision {{ item.sourceRevision }}</p><p class="pt-2 font-medium">共享源码变化</p><p class="text-muted-foreground">下方是任务开始后的共享目录差异，可能包含其他会话操作。</p></div><p v-if="!session.task" class="p-3 text-xs leading-6 text-muted-foreground">任务开始后显示源码变更与差异。</p><ChangedFiles v-else :changes="changes" :selected="selected" @select="select" /><FileDiff v-if="diff" :diff="diff" :mod-id="session.modId" /><p v-if="error" role="alert" class="p-3 text-xs text-destructive">{{ error }}</p></ScrollArea>
      </TabsContent>
      <ScrollArea v-show="selectedTab === 'logs'" class="min-h-0 flex-1">
        <TabsContent value="logs"><div class="flex items-center gap-2 p-3"><Button v-if="session.task" size="sm" variant="outline" @click="openLogs">打开任务全文日志</Button><Button size="sm" variant="ghost" @click="refreshLogs">刷新</Button></div><p class="px-3 pb-3 text-[11px] text-muted-foreground">会话运行日志 · 最近 200 条 · 每 3 秒刷新</p><p v-if="logError" role="alert" class="p-3 text-xs text-destructive">{{ logError }}</p><p v-if="!runtimeLogs.length" class="p-3 text-xs text-muted-foreground">暂无已保存的运行日志。更新前的运行摘要不会补录，既有命令日志仍可通过上方按钮查看。</p><div v-for="(entry, index) in runtimeLogs" :key="`${entry.timestamp}-${index}`" class="border-b px-3 py-2 text-[11px] leading-5"><p class="text-muted-foreground">{{ new Date(entry.timestamp).toLocaleString() }} · {{ entry.taskId ? `任务 ${entry.taskId.slice(0, 8)}` : '连接' }}</p><pre class="mt-1 whitespace-pre-wrap break-all">{{ entry.message }}</pre></div><template v-if="session.diagnostics.length"><p class="px-3 pt-4 text-xs font-medium">当前连接诊断</p><pre v-for="(line, index) in session.diagnostics" :key="index" class="border-b px-3 py-2 font-mono text-[11px] leading-5 whitespace-pre-wrap break-all">{{ line }}</pre></template></TabsContent>
      </ScrollArea>
      <TabsContent value="artifacts" class="flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden">
        <div v-if="session.modId" class="shrink-0 p-3"><Button as-child class="w-full" size="sm" variant="outline"><RouterLink :to="`/mods/${session.modId}/versions`">版本与安装管理</RouterLink></Button></div>
        <div class="min-h-0 flex-1 overflow-y-auto" role="region" aria-label="产物版本列表">
          <p v-if="!artifactProject?.versions.length" class="p-3 text-xs leading-6 text-muted-foreground">尚无成功交付版本。</p>
          <button v-for="version in [...(artifactProject?.versions ?? [])].reverse()" :key="version.versionId" type="button" class="block w-full border-b p-3 text-left text-xs transition-colors hover:bg-muted/50 focus-visible:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring" :aria-label="`查看版本 ${version.versionId.slice(0, 8)} 的详情`" :title="`查看版本 ${version.versionId} 的详情`" @click="viewArtifact(version.versionId)">
            <span class="flex flex-wrap items-center justify-between gap-2"><span class="font-mono">{{ version.versionId.slice(0, 8) }}</span><span v-if="installedVersionIds.has(version.versionId)" class="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[11px] text-emerald-700 dark:text-emerald-400">当前安装</span></span>
            <span class="mt-2 block text-muted-foreground">{{ version.files.length }} 个文件 · {{ new Date(version.createdAt).toLocaleString() }}</span>
          </button>
        </div>
      </TabsContent>

    </Tabs>
    <ArtifactPanel v-if="selectedArtifact" v-model:open="artifactDetailsOpen" :version="selectedArtifact" />
  </section>
</template>
